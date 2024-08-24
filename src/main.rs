use clap::Parser;
use reqwest::Client;
use serde_json::Value;
use serde_json::json;
use std::error::Error;
use std::fs;
use std::io;
 
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
        _ => panic!("idk")
    };
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
    
    // Extract and print `package_pname` from the response
    if let Some(hits) = response_json["hits"]["hits"].as_array() {
        for hit in hits {
            if let Some(_score) = hit.get("_score") {
                println!("Package score: {}", _score);
            }
            match hit["_source"].get("package_attr_name") {
                Some(package_attr_name) => println!("Package Name: {}", package_attr_name),
                None => continue,
            }
        }
    } else {
            eprintln!("Unexpected response format.");
    };
    
    Ok(())
}

