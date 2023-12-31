/*
 Handling of commands, arguments.
 Interacts with config module to
 gather/store configuration.
*/

use std::{
    env,
    fs::{self, File},
    process::Command,
    thread,
    time::Duration,
};

use colored::Colorize;
use maplit::hashmap;

use crate::{
    compiler::{CompType, Compiler},
    config::ConfigFile,
    creator::Project,
    initiator,
    tips::*,
    util::{self, throw_error, ErrorType},
};

const INTRO: &str = r#"
This is the Surtur build tool for C

The most important commands are:
- new // create a new surtur C project
- run // compiles and executes your program
- build // compiles your program
- help // use for additional help
- add <name> // adds the specified library
- remove <name> // removes the specified library
- update // use this when making changes to the project.lua file
- init // initialize a surtur C project
"#;

pub struct Cli {
    args: Vec<String>,
    cfg: Option<ConfigFile>,
    cur_dir: String,
}

impl Cli {
    pub fn new() -> Self {
        let cur_dir_raw = match env::current_dir() {
            Ok(dir) => dir,
            Err(_) => throw_error(
                ErrorType::MISC,
                "Failed to get current directory.
        Please report this issue here https://github.com/Thepigcat76/surtur/issues",
                None,
            ),
        };

        let cur_dir = match cur_dir_raw.to_str() {
            Some(cur_dir) => cur_dir.to_string(),
            None => throw_error(
                ErrorType::MISC,
                "Failed to convert current directory to string.
        Please report this issue here https://github.com/Thepigcat76/surtur/issues",
                None,
            ),
        };


        let path = format!("{}/project.lua", cur_dir,);

        let mut file = match File::open(&path) {
            Ok(file) => Some(file),
            Err(_) => None,
        };

        let blue_line = "|".bright_blue();

        let cfg = match &mut file {
            Some(cfg_file) => Some(ConfigFile::from(cfg_file)),
            None => None,
        };

        Self {
            cfg,
            cur_dir,
            args: env::args().collect(),
        }
    }

    pub fn execute(&self) {
        let cmd_tips = hashmap! {
            "uninstall" => "remove",
            "install" => "add",
            "compile" => "build",
            "execute" => "run",
            "create" => "new",
            "package" => "bundle"
        };

        let first_arg = self.args.get(1);
        let second_arg = self.args.get(2);

        let mut matched = false;

        match first_arg {
            Some(arg) => match arg.as_str() {
                "new" => {
                    self.create_proj(match second_arg {
                        Some(arg) => arg,
                        None => throw_error(
                            ErrorType::CREATION,
                            "Failed to set project name",
                            Some(get_tip(Tip::MissingProjName)),
                        ),
                    });
                }
                "run" => {
                    match second_arg {
                        Some(arg) => match arg.as_str() {
                            "-dbg" | "-d" => self.run_c(true),
                            _ => throw_error(
                                ErrorType::EXECUTION,
                                "Invalid argument for running the program",
                                Some(get_tip(Tip::InvalidRunArg)),
                            ),
                        },
                        None => self.run_c(false),
                    }
                }
                "build" => {
                    let mut actual_args = self.args.clone();
                    actual_args.remove(0);

                    let mut is_release = false;
                    let mut comp_type = CompType::EXE;
                    actual_args.iter().for_each(|arg| match arg.as_str() {
                        "-exe" | "-e" => comp_type = CompType::EXE,
                        "-asm" | "-a" | "-s" => comp_type = CompType::ASM,
                        "-obj" | "-o" => comp_type = CompType::OBJ,
                        "-release" | "-r" => is_release = true,
                        _ => throw_error(
                            ErrorType::BUILD,
                            "Invalid argument",
                            Some(get_tip(Tip::InvalidBuildArg)),
                        ),
                    });
                    println!("{:?}, {}", &actual_args, is_release);
                    self.build_c(comp_type, false, is_release);
                }
                "init" => {
                    let root_dir = match env::current_dir() {
                        Ok(root) => match root.to_str() {
                            Some(str_root) => str_root.to_string(),
                            None => throw_error(
                                ErrorType::INITIALIZATION,
                                "Failed to convert root directory to a string",
                                None,
                            ),
                        },
                        Err(_) => throw_error(
                            ErrorType::INITIALIZATION,
                            "Failed to get root directory",
                            None,
                        ),
                    };
                    let proj = Project::new(&root_dir);

                    println!("{:?}", proj);

                    initiator::init_proj(&proj);
                }
                "add" => {

                }
                _ => {
                    for (key, val) in cmd_tips {
                        if arg.as_str() == key {
                            matched = true;
                            println!("`{}` is not a valid argument. Use `{}` instead", key, val);
                            break;
                        }
                    }
                    if !matched {
                        println!(
                            "`{}` is not a valid argument. Use `help` to see all valid arguments",
                            arg
                        )
                    }
                }
            },
            None => println!("{}", INTRO),
        }
    }

    // TODO: Extremly hacky please fix
    fn run_c(&self, enable_dbg: bool) {
        let root_name = util::root_dir_name(&self.cur_dir);
        let executable_path = format!("./build/{}", root_name);

        {
            let mut file_available = true;

            match fs::remove_file(format!("build/{}", root_name)) {
                Ok(()) => (),
                Err(_) => (),
            }

            while file_available {
                if fs::metadata(&executable_path).is_err() {
                    file_available = false;
                } else {
                    // Sleep for a short duration before checking again
                    thread::sleep(Duration::from_millis(500)); // 500 milliseconds
                }
            }
        }

        self.build_c(CompType::EXE, enable_dbg, false);

        let mut file_available = false;

        while !file_available {
            if fs::metadata(&executable_path).is_ok() {
                file_available = true;
            } else {
                // Sleep for a short duration before checking again
                thread::sleep(Duration::from_millis(500)); // 500 milliseconds
            }
        }

        if file_available {
            // Create a Command to run the executable
            let mut cmd = Command::new(format!("{}", &executable_path));
            cmd.output().expect("Failed to run executable");

            match cmd.status() {
                Ok(status) => {
                    if status.success() {
                        println!("Program executed successfully.");
                    } else {
                        eprintln!("Command failed with exit code: {}", status);
                    }
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                }
            }
        } else {
            eprintln!("Timed out waiting for the executable file to become available.");
        }
    }

    fn build_c(&self, comp_type: CompType, enable_dbg: bool, is_release: bool) {
        let blue_line = "|".bright_blue();
        let path = format!("{}/project.lua", self.cur_dir);
        
        let missing_cfg_file = format!(
            r#"
    {} Could not locate config file at {}
    {} 
    {} Use 
    {} {}> {} init
    {} To create a new config file
    "#,
            blue_line,
            path,
            blue_line,
            blue_line,
            blue_line,
            self.cur_dir,
            "surtur".yellow(),
            blue_line,
        );
        println!("{}", self.cur_dir);
        let mut builder = Compiler::new(&self.cur_dir);
        let cfg = match &self.cfg {
            Some(cfg) => cfg,
            None => throw_error(ErrorType::EXECUTION, "Missing project config file", Some(missing_cfg_file)),
        };
        builder
            .build(comp_type, cfg.c_std, enable_dbg, is_release)
            .expect("Failed to build project");
    }

    fn create_proj(&self, name: &str) {
        let project = Project::new(name);
        project.create();
    }
}
