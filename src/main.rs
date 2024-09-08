mod args;

use args::build_cli;
use clap::{command, Arg, ArgGroup, Command, ValueHint};
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::fs;
use std::fs::{OpenOptions, File};
use std::io::{Read, Write};
use std::path::Path;
use inquire::{error::InquireError, Select};
use serde::Deserialize;

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
        if let Some(value) = sub_matches.get_one::<String>("create") {
                profile_create(value.to_string());
        }
        if let Some(value) = sub_matches.get_one::<String>("list") {
            profile_list();
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
            println!("Test passed on line: {}", line_number);
            new_contents.push_str(&format!("  {}\n", packagename));

            if line.contains("with pkgs") {
                println!("pkgs"); // todo!
            }
        }
    }
    // save to file
    let mut file = OpenOptions::new().write(true).truncate(true).open("config.nix")?;
    file.write_all(new_contents.as_bytes())?;

    println!("done");
    Ok(())
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

fn profile_create(mut name: String) {
    let source_file = "profile_configs/default.toml"; // clones the default.toml when creating a
                                                      // new config file
    let filepath = String::from("profile_configs"); // todo! this sould be in the users .config
                                                    // currently in the local project location
    name.push_str(".toml");
    let name_filepath = Path::new(&filepath).join(name);
    let entries = fs::copy(source_file, name_filepath);
}

fn profile_list() {
    let filepath = fs::read_dir("profile_configs").unwrap(); // todo! this hsould be in the users
                                                             // .config folder not project folder

    for path in filepath {
        println!("Name: {}", path.unwrap().path().display())
    }
}
