use clap::{command, Arg, ArgGroup, Command};

pub fn build_cli() -> Command {
    command!()
        .about("wolfpack")
        .subcommand(
            Command::new("packages")
                .arg(
                    Arg::new("search")
                        .short('s')
                        .long("search")
                        .help("Searches for nix package based on the name")
                        .conflicts_with_all(["install", "search-install"])
                )
                .arg(
                    Arg::new("install")
                        .short('i')
                        .long("install")
                        .help("Writes package name to config file")
                        .conflicts_with_all(["search", "search-install"])
                )
                .arg(
                    Arg::new("search-install")
                        .short('x')
                        .long("search-install")
                        .aliases(["si", "is"])
                        .help("Searches packages and installs selected package")
                        .conflicts_with_all(["search", "install"])
                )
                .arg(
                    Arg::new("profile-selection")
                        .short('p')
                        .long("profile-select")
                        .aliases(["select-profile", "select", "profile-selection"])
                        .help("select the profile used for this action")
                        .requires_all(["search", "install", "search-install"])
                )
                .arg(
                    Arg::new("create")
                        .short('c')
                        .long("create")
                        .help("Create a new profile")
                        .exclusive(true)
                )
                .arg(
                    Arg::new("list")
                        .short('l')
                        .long("list")
                        .help("List all profiles")
                        .exclusive(true)
                )
        )
}

