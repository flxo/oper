#[macro_use]
extern crate clap;
extern crate cursive;
extern crate indicatif;
extern crate num_cpus;
#[macro_use]
extern crate lazy_static;

mod model;
mod styles;
mod ui;
mod utils;
mod views;

use clap::{App, Arg};
use model::{MultiRepoHistory, Repo};
use std::convert::Into;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Arc;
use utils::{find_project_file, find_repo_base_folder};

const MAX_NUMBER_OF_THREADS: usize = 18; //tests on a 36 core INTEL Xeon showed that parsing becomes slower again if more than 18 threads are used

fn main() -> Result<(), String> {
    let original_cwd = env::current_dir().expect("cwd not found");
    let matches = App::new("oper")
        .version("0.1.0")
        .author("Florian Bramer <elektronenhirn@gmail.com>")
        .about("git-repo history tool")
        .arg(
            Arg::with_name("days")
                .short("d")
                .long("days")
                .value_name("days")
                .help("include history of the last <n> days")
                .default_value("100")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("author")
                .short("a")
                .long("author")
                .value_name("pattern")
                .help(
                    "only include commits where author's name contains <pattern> (case insensitive)",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name("message")
                .short("m")
                .long("message")
                .value_name("pattern")
                .help("only include commits where message contains <pattern> (case insensitive)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("cwd")
                .short("C")
                .long("cwd")
                .value_name("cwd")
                .help("change working directory (mostly useful for testing)")
                .default_value(original_cwd.to_str().unwrap())
                .takes_value(true),
        )
        .get_matches();

    let days = value_t!(matches.value_of("days"), u32).unwrap_or_else(|e| e.exit());
    let classifier = model::Classifier::new(
        days,
        matches.value_of("author"),
        matches.value_of("message"),
    );
    let cwd = Path::new(matches.value_of("cwd").unwrap());

    do_main(&classifier, cwd).or_else(|e| Err(e.description().into()))
}

fn do_main(classifier: &model::Classifier, cwd: &Path) -> Result<(), io::Error> {
    env::set_current_dir(cwd)?;
    rayon::ThreadPoolBuilder::new()
        .num_threads(std::cmp::min(num_cpus::get(), MAX_NUMBER_OF_THREADS))
        .build_global()
        .unwrap();

    let project_file = File::open(find_project_file()?)?;
    let repos = repos_from(&project_file)?;

    let history = MultiRepoHistory::from(repos, &classifier)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.description()))?;

    ui::show(history);

    Ok(())
}

fn repos_from(project_file: &std::fs::File) -> Result<Vec<Arc<Repo>>, io::Error> {
    let mut repos = Vec::new();

    let base_folder = find_repo_base_folder()?;
    for project in BufReader::new(project_file).lines() {
        let rel_path = project.expect("project.list read error");
        let abs_path = base_folder.join(&rel_path);
        repos.push(Arc::new(Repo::from(abs_path, rel_path)));
    }

    Ok(repos)
}
