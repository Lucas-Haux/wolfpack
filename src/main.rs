use clap::{command, Arg, ArgGroup, Command, ValueHint};
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::fs;
use std::fs::{OpenOptions};
use std::io::{Read, Write};
use inquire::{error::InquireError, Select};

fn main() {
    let match_result = command!()
        .about("wolfpack")
        .subcommand(
            Command::new("packages") // subcommands for packages
                .arg(
                    Arg::new("search") // search for package
                        .short('s')
                        .long("search")
                        .help("Searches for nix package based on the name")
                )
                .arg(
                    Arg::new("install") // install package
                        .short('i')
                        .long("install")
                        .help("Writes package name to config file")
                )
                .arg(
                    Arg::new("search-install") // search and installs
                        .short('x')
                        .long("search-install")
                        .aliases(["si", "is"])
                        .help("Searches packages and installs selected package")
                )
        )
        .get_matches();

    // check if user used subcommand packages
    if let Some(sub_matches) = match_result.subcommand_matches("packages") {
        if let Some(search_value) = sub_matches.get_one::<String>("search") {
            query_search(search_value.to_string());
        } 
        if let Some(search_value) = sub_matches.get_one::<String>("install") {
            install(search_value.to_string(), false);
        } 
        if let Some(search_value) = sub_matches.get_one::<String>("search-install") {
            install(search_value.to_string(), true);
        } 
    }
}

// search and add package to file
fn install(search: String, search_before_install: bool) -> Result<Vec<String>, Box<dyn Error>> {
    let mut answer = String::new();
    // seach install or just install
    if search_before_install == true {
        //search for query
        let options = query_search(search);
        //Convert to &str
        let strings = options.expect("REASON");
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

    write_to_file(answer);
    let test = Vec::new();
    Ok(test)
} 

// write to file
fn write_to_file(packagename: String) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open("config.nix")?;
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
                println!("pkgs");
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
async fn query_search(search: String) -> Result<Vec<String>, Box<dyn Error>> {
    let client = Client::new();
    let url = "https://search.nixos.org/backend/latest-42-nixos-24.05/_search";
    let json_data = fs::read_to_string("query.json")?;
    let mut query_body: Value = serde_json::from_str(&json_data)?;

    // Update the field 
    query_body["query"]["bool"]["must"][0]["dis_max"]["queries"][0]["multi_match"]["query"] = json!(search);
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

