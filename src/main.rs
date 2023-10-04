use std::cmp::min;
use std::fs;
use std::fs::File;
#[cfg(unix)] use std::fs::Permissions;
use std::io::Write;
use std::path::PathBuf;
use clap::{Parser, Subcommand};
use futures_util::stream::StreamExt;
use kdam::{BarExt, Column, RichProgress, Spinner, tqdm};
use lazy_static::lazy_static;

/// The Djinn Toolchain Manager
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Installs a version of Djinn and adds it to the PATH
    Install {
        /// The version of the Djinn toolchain to install; leave blank to install the latest version
        version: Option<String>
    },
    /// Lists the toolchains available
    List,
    /// Switches to a particular Djinn toolchain for compilation.
    Use {
        /// The version of the Djinn toolchain to install
        version: String
    }
}

lazy_static! {
    static ref PATH: PathBuf = {
        let mut dir = dirs::data_local_dir().unwrap();
        dir.push(".djinn/");
        fs::create_dir_all(&dir).unwrap();
        dir
    };
}



#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Install { version } => {
            macro_rules! resolve {
                ($base: ident, $x: expr) => {{
                    let mut temp = $base.clone();
                    temp.push($x);
                    temp
                }};
            }
            let instance = octocrab::instance();
            let repos = instance.repos("gsayson", "djinn");
            let releases = repos.releases();
            let release = match version {
                None => {
                    println!("Installing latest Djinn version");
                    releases.get_latest().await
                }
                Some(ref v) => {
                    println!("Installing Djinn version '{v}'");
                    releases.get_by_tag(&*("v".to_owned() + v)).await
                }
            };
            match release {
                Ok(release) => {
                    let cli_jar_url = (&release.assets[0].browser_download_url).clone();
                    match reqwest::get(cli_jar_url.clone()).await {
                        Ok(response) => {
                            let total_size = response
                                .content_length()
                                .unwrap_or(10000);
                            let mut pb = RichProgress::new(
                                tqdm!(
                                    total = total_size as usize,
                                    unit_scale = true,
                                    unit_divisor = 1024,
                                    unit = "B"
                                ),
                                vec![
                                    Column::Spinner(Spinner::new(
                                        &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
                                        80.0,
                                        1.0,
                                    )),
                                    Column::Text("[bold blue]?".to_owned()),
                                    Column::Animation,
                                    Column::Percentage(1),
                                    Column::Text("•".to_owned()),
                                    Column::CountTotal,
                                    Column::Text("•".to_owned()),
                                    Column::Rate,
                                    Column::Text("•".to_owned()),
                                    Column::RemainingTime,
                                ],
                            );
                            let toolchain_dir = {
                                let mut temp = PATH.clone();
                                temp.push(format!("toolchains/{}/", release.tag_name));
                                temp
                            };
                            fs::create_dir_all(&toolchain_dir).unwrap();
                            let path = resolve!(toolchain_dir, format!("{:x}.jar", release.id.0));
                            let mut file = File::create(&path).unwrap();
                            let mut downloaded: u64 = 0;
                            let mut stream = response.bytes_stream();
                            pb.render();
                            while let Some(item) = stream.next().await {
                                let chunk = item.unwrap();
                                file.write_all(&chunk).unwrap();
                                let new = min(downloaded + (chunk.len() as u64), total_size);
                                downloaded = new;
                                pb.pb.update_to(downloaded as usize).unwrap();
                            }
                            println!();
                            fs::write(resolve!(toolchain_dir, "djinn-cli.bat"), format!("@echo off\njava -jar {}", path.display())).unwrap();
                            fs::write(resolve!(toolchain_dir, "djinn-cli.sh"), format!("java -jar {}", path.display())).unwrap();
                            #[cfg(unix)] {
                                use std::os::unix::fs::PermissionsExt;
                                set_permissions(resolve!(toolchain_dir, "djinn-cli.sh"), Permissions::from_mode(755)).unwrap();
                            }
                            println!("Installed Djinn CLI at {}", toolchain_dir.display());
                        }
                        Err(err) => {
                            eprintln!("Unable to install version '{}'", release.tag_name.replace("v", ""));
                            eprintln!("{err}");
                        }
                    };
                }
                Err(err) => {
                    eprintln!("Unable to install version '{}'", version.unwrap_or("latest".to_owned()));
                    eprintln!("{err}");
                }
            }
        }
        Commands::List => {
            let mut x = PATH.clone();
            x.push("toolchains/");
            println!("toolchain home:");
            println!("-> {}\n", PATH.display());
            println!("all installed versions:");
            fs::read_dir(x).unwrap()
                .filter(Result::is_ok)
                .map(|f| f.unwrap().file_name().to_string_lossy().replace("v", ""))
                .for_each(|v| {
                    println!("-> djinn toolchain {}", v);
                });
            println!();
        }
        Commands::Use { version } => {
            macro_rules! resolve {
                ($base: ident, $x: expr) => {{
                    let mut temp = $base.clone();
                    temp.push($x);
                    temp
                }};
            }
            let mut x = PATH.clone();
            x.push("toolchains/");
            let c = fs::read_dir(&x).unwrap()
                .filter(Result::is_ok)
                .map(Result::unwrap)
                .map(|f| f.file_name())
                .map(|f| f.into_string().unwrap())
                .map(|f| f.replacen("v", "", 1))
                .any(|v| v == version);
            if c {
                x.push(format!("{}/", {
                    let mut s = String::from('v');
                    s += &*version;
                    s
                }));
                let mut y = x.clone();
                fs::write(PATH.join("djinn-cli.bat"), format!("@echo off\n{}", {
                    y.push("djinn-cli.bat");
                    y.display()
                })).unwrap();
                let mut y = x.clone();
                fs::write(PATH.join("djinn-cli.sh"), format!("{}", {
                    y.push("djinn-cli.sh");
                    #[cfg(unix)] {
                        use std::os::unix::fs::PermissionsExt;
                        set_permissions(&y, Permissions::from_mode(755)).unwrap();
                    }
                    y.display()
                })).unwrap();
                println!("Using Djinn toolchain {version}");
            } else {
                eprintln!("No such version '{version}' is installed");
                std::process::exit(1);
            }
        }
    }
}
