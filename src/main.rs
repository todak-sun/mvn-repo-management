use git2::Repository;

use std::env;
use xml::reader::{EventReader, XmlEvent};

use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = if args.len() == 1 { "." } else { &args[1] };

    let repo = match Repository::open(path) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open repository: {}", e),
    };

    let mut options = git2::StatusOptions::new();
    options.include_untracked(true);

    let statuses = match repo.statuses(Some(&mut options)) {
        Ok(statuses) => statuses,
        Err(e) => panic!("failed to get statuses: {}", e),
    };

    let uncommitted_files: Vec<PathBuf> = statuses
        .iter()
        .filter_map(|s| s.path().map(|p| PathBuf::from(p)))
        .collect();

    let mut modules: Vec<String> = vec![];
    let mut dependencies: HashMap<String, Vec<String>> = HashMap::new();

    let file = File::open(format!("{}/pom.xml", path)).unwrap();
    let file = EventReader::new(file);
    let mut in_module = false;

    for e in file {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) if name.local_name == "module" => {
                in_module = true;
            }
            Ok(XmlEvent::Characters(s)) if in_module => {
                if !modules.contains(&s) {
                    modules.push(s.clone());
                }
                let dep_file = File::open(format!("{}/{}/pom.xml", path, s)).unwrap();
                let dep_file = EventReader::new(dep_file);
                let mut in_dependency = false;
                let mut current_dependencies: Vec<String> = vec![];

                for dep in dep_file {
                    match dep {
                        Ok(XmlEvent::StartElement { name, .. })
                            if name.local_name == "artifactId" =>
                        {
                            in_dependency = true;
                        }
                        Ok(XmlEvent::Characters(dep_name)) if in_dependency => {
                            current_dependencies.push(dep_name);
                        }
                        Ok(XmlEvent::EndElement { name }) if name.local_name == "artifactId" => {
                            in_dependency = false;
                        }
                        _ => {}
                    }
                }

                dependencies.insert(s, current_dependencies);
            }
            Ok(XmlEvent::EndElement { name }) if name.local_name == "module" => {
                in_module = false;
            }
            _ => {}
        }
    }

    for module in modules {
        let files: Vec<&PathBuf> = uncommitted_files
            .iter()
            .filter(|file| file.starts_with(&module))
            .collect();

        if files.is_empty() {
            continue;
        }

        let mut depend_on: Vec<&String> = dependencies
            .iter()
            .filter(|(_, deps)| deps.contains(&module))
            .map(|(other_module, _)| other_module)
            .collect();

        println!("Module [{:?}]", module);
        println!("\nUncommitted Files.");
        for filename in files {
            println!("- {:?}", filename);
        }
        println!("\nFollow modules depend on {:?}. You should check", module);
        depend_on.sort_by(|a, b| a.cmp(b));
        for dep in depend_on {
            println!("- {:?}", dep);
        }
    }
}
