use clap::Parser;
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::fs;
use std::fs::{File, OpenOptions, read_to_string, write};
use std::io::{self, BufReader, Read, Write, prelude::*};

#[derive(Parser)]
#[clap(author, version)]
struct Args {
    #[clap(short, long, value_parser)]
    wolfpack: String,

    #[clap(value_parser)]
    query: String,

}

fn main() {
    let args = Args::parse();
    let result = match args.wolfpack.as_str() {
        "s" => query_search(args.query),
        //"i" => install(),
        _ => panic!("idk")
    };
    let testpackage = String::from("testpackage");
    write_to_file(testpackage);
}

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

 
#[tokio::main]
async fn query_search(search: String) -> Result<(), Box<dyn Error>> {
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
    
    // Extract and print the packagename from the response
    if let Some(hits) = response_json["hits"]["hits"].as_array() {
        for hit in hits {
            if let Some(_score) = hit.get("_score") {
                println!("Package score: {}", _score);
            }
            if let Some(package_attr_name) = hit["_source"].get("package_attr_name") {
                println!("Package name: {}", package_attr_name);
            }
        }
    } else {
        eprintln!("Unexpected response format.");
    };
    
    Ok(())
}

