mod args;

use args::build_cli;
use clap::{command, Arg, ArgGroup, Command, ValueHint};
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::fs;
use std::fs::{OpenOptions, File};
use std::io::{Read, Write, BufReader, BufRead};
use std::io;
use std::path::Path;
use inquire::{error::InquireError, Select};
use serde::Deserialize;
use dialoguer::Confirm;
use std::process::Command as OtherCommand;

// Structs for config.toml files
#[derive(Deserialize, Debug)]
struct Config {
    query: QueryConfig, 
    nix: NixConfig,
}

#[derive(Deserialize, Debug)]
struct QueryConfig {
    length: i8,
    url : String,
}

#[derive(Deserialize, Debug)]
struct NixConfig {
    location: String,
    rebuildSwitch: bool,
    rebuildCommand: String,
}

fn main() {
    let mut profile = String::from("profile_configs/default.toml"); 
    // command arguments
    let match_result = build_cli().get_matches();

    // check if user used subcommand packages
    if let Some(sub_matches) = match_result.subcommand_matches("packages") {
        if let Some(value) = sub_matches.get_one::<String>("profile-selection") {
            profile = value.to_string(); //defaults to default.toml
            profile.push_str(".toml");
            profile.insert_str(0, "profile_configs/"); // todo! change to sysytem profile location 
        }
        let config_content = fs::read_to_string(profile.clone()).expect("Unable to read file");
        // Parse the content
        let config: Config = toml::from_str(&config_content).expect("Unable to parse");

        if let Some(value) = sub_matches.get_one::<String>("search") {
            query_search(value.to_string(), &config);
        } 
        if let Some(value) = sub_matches.get_one::<String>("install") {
            install(value.to_string(), false, &config); // dont search for packages before install
        } 
        if let Some(value) = sub_matches.get_one::<String>("search-install") {
            install(value.to_string(), true, &config); // search for packages before install
        } 
        if let Some(value) = sub_matches.get_one::<String>("remove-package") {
            remove_package(value.to_string(), &config);
        }
        if let Some(value) = sub_matches.get_one::<String>("create-profile") {
            profile_create(value.to_string());
        }
        if let Some(value) = sub_matches.get_one::<String>("list-profiles") {
            profile_list();
        } 
        if let Some(value) = sub_matches.get_one::<String>("remove-profile") {
            profile_remove(value.to_string());
        }
    }
}

// search and add package to file
fn install(search: String, search_before_install: bool, profile: &Config)  {
    let mut answer = String::new();
    // seach install or just install
    if search_before_install == true {
        let query_answer = query_search(search, &profile);
        //Convert to &str
        let strings = query_answer.expect("REASON");
        let string_refs: Vec<&str> = strings.iter().map(|s| s.as_str()).collect();
        //inquire select
        let ans: Result<&str, InquireError> = Select::new("Here are the results, which package do you want to install", string_refs).prompt();
        match ans {
            Ok(choice) => { 
                println!("Installing {}", choice);
                answer = choice.to_string();
            },
            Err(_) => println!("There was an error, please try again"),
        }
        println!("answer: {}", answer);
    } else {
        answer = search;
    };

    write_to_file(answer, &profile);

    // rebuildSwitch
    if profile.nix.rebuildSwitch == true {
        let confirmation = Confirm::new()
            .with_prompt("Do you want to run nixos rebuild switch?")
            .interact()
            .unwrap();

        if confirmation == true {
            println!("Running command defined in your profile");
            // Get the command arguments from the toml file
            let args: &Vec<&str> = &profile.nix.rebuildCommand.split_whitespace().collect();

            if let Some((command, arguments)) = args.split_first() {
                let output = OtherCommand::new(command)
                    .args(arguments)
                    .output()
                    .expect("Failed to execute command");

                println!("Status: {}", output.status);

                // Check if the command was successful
                if output.status.success() {
                    println!("Command executed successfully!");
                } else {
                    eprintln!("Command failed to execute.");
                    eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
                }
            }
        } else { println!("Not running nixos rebuild"); }
    }
} 

// write to file
fn write_to_file(packagename: String, profile: &Config) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&profile.nix.location)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let mut new_contents = String::new();

    let mut line_number = 0;
    for line in contents.lines() {
        line_number += 1;
        new_contents.push_str(line);
        new_contents.push('\n');

        if line.contains("environment.systemPackages") {
            // get the amount of spaces on the environment.systemPackages line
            let leading_spaces = count_leading_spaces(line);
            let mut spaces = String::new();
            let mut num = leading_spaces as i16;
            while num > 0 {
                spaces.push_str(" ");
                num -= 1;
            }
            spaces.push_str("  "); // add two more spaces then environement.systemPackages line

            if line.contains("with pkgs") {
                new_contents.push_str(&format!("{}{}\n", spaces, packagename));
            } else {
                new_contents.push_str(&format!("{}pkgs.{}\n", spaces, packagename)); //add pkgs. if
                                                                                     //it needs it
            }
        }
    }
    // save to file
    let mut file = OpenOptions::new().write(true).truncate(true).open("config.nix")?;
    file.write_all(new_contents.as_bytes())?;

    println!("done");
    Ok(())
}

 fn count_leading_spaces(line: &str) -> usize {
    line.chars()
        .take_while(|c| c.is_whitespace() && *c == ' ')
        .count()
}
 
// Search
#[tokio::main]
async fn query_search(search: String, profile: &Config) -> Result<Vec<String>, Box<dyn Error>> {
    let client = Client::new();
    let url = &profile.query.url;
    let json_data = fs::read_to_string("query.json")?;
    let mut query_body: Value = serde_json::from_str(&json_data)?;

    // Update the field 
    query_body["query"]["bool"]["must"][0]["dis_max"]["queries"][0]["multi_match"]["query"] = json!(search);
    query_body["query"]["bool"]["must"][0]["dis_max"]["queries"][0]["multi_match"]["_name"] = json!(format!("multi_match_{}", search));
    query_body["query"]["bool"]["must"][0]["dis_max"]["queries"][1]["wildcard"]["package_attr_name"]["value"] = json!(format!("*{}*", search)); // doesnt work 
    query_body["size"] = json!(&profile.query.length);
    fs::write("query.json", serde_json::to_string_pretty(&query_body)?)?;

    // Make the request with API key included in the headers
    let response = client
        .post(url)
        .header("Authorization", "Bearer YVdWU0FMWHBadjpYOGdQSG56TDUyd0ZFZWt1eHNmUTljU2g=")
        .json(&query_body)
        .send()
        .await?;
    
    // Get the raw JSON response as a Value
    let response_json: Value = response.json().await?;
    
    let mut options: Vec<&str> = Vec::new();
    // Extract and print the packagename from the response
    if let Some(hits) = response_json["hits"]["hits"].as_array() {
        for hit in hits {
            if let Some(package_attr_name) = hit["_source"].get("package_attr_name").and_then(|v| v.as_str()) {
                println!("Package name: {}", package_attr_name);
                options.push(package_attr_name.trim());
            }
            if let Some(package_description) = hit["_source"].get("package_description") {
                println!("{}\n", package_description);
            }
        }
    } else {
        eprintln!("Unexpected response format.");
    };
    let string_options: Vec<String> = options.iter().map(|&s| s.to_string()).collect();
    Ok(string_options)
}

fn remove_package (mut packageName: String, profile: &Config) -> io::Result<()> {
    let file = File::open(&profile.nix.location)?;
    let reader = BufReader::new(file);
    let mut package_exists = false;

    let mut lines_to_keep = Vec::new();

    let pks_package_name = format!("pkgs.{}", packageName);
    // Read through the file and collect lines that don't contain the target line
    for line in reader.lines() {
        let line = line?;
        if line.trim() == packageName || line.trim() == pks_package_name {
            println!("Found it, removing the line.");
            continue; // Skip this line
            package_exists = true
        }
        lines_to_keep.push(line);
    }
    if package_exists == false {
        println!("Cant find package {} in {}", packageName, &profile.nix.location);
        println!("Nothing was removed");
    }

    // Write the remaining lines back to the file
    let mut file = File::create(&profile.nix.location)?;
    for line in lines_to_keep {
        writeln!(file, "{}", line)?;
    }

    Ok(())    
}

fn profile_create(mut filename: String) {
    let source_file = "profile_configs/default.toml"; // clones the default.toml when creating a
                                                      // new config file
    let configpath = String::from("profile_configs"); // todo! this sould be in the users .config
                                                    // currently in the local project location
    if !filename.ends_with(".toml") {
        filename.push_str(".toml");
    }
    let name_filepath = Path::new(&configpath).join(filename);

}

fn profile_list() {
    let filepath = fs::read_dir("profile_configs").unwrap(); // todo! this hsould be in the users
                                                             // .config folder not project folder

    for path in filepath {
        println!("Name: {}", path.unwrap().path().display())
    }
}

fn profile_remove(mut filename: String) {
    let configpath = String::from("profile_configs");
    
    // make is you cant delete the default profile
    if filename == "default" || filename == "default.toml" {
        panic!("You cant delete the default profile");
    }
    
    // add .toml at the end of the file name if it doesnt have it already
    if !filename.ends_with(".toml") {
        filename.push_str(".toml");
    }

    // make the fullpath, config + filename
    let fullpath = Path::new(&configpath).join(&filename); 
    println!("fullpath: {:#?}", fullpath);

    // check if profile exists
    if fullpath.exists() {
        match fs::remove_file(fullpath) {
            Ok(_) => println!("File removed successfully."),
            Err(e) => println!("Failed to remove file: {}", e),
        }
    } else {
        println!("File does not exists");
    }
}
