extern crate reqwest;
extern crate serde;
extern crate serde_json;

use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize)]
struct DropboxCredentials {
    client_id: String,
    client_secret: String,
}
fn get_credentials() -> Result<DropboxCredentials, Box<dyn std::error::Error>> {
    let config_file = std::fs::read_to_string("config.json")?;
    let credentials: DropboxCredentials = serde_json::from_str(&config_file)?;
    Ok(credentials)
}

async fn get_access_token(
    credentials: &DropboxCredentials,
) -> Result<String, Box<dyn std::error::Error>> {
    let auth_url = format!(
        "https://api.dropbox.com/oauth2/token?client_id={}&client_secret={}&grant_type=client_credentials",
        credentials.client_id,
        credentials.client_secret
    );

    let response = reqwest::Client::new()
        .post(&auth_url)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .send()
        .await?
        .text()
        .await?;

    let json_data: Value = serde_json::from_str(&response)?;
    let access_token = json_data["access_token"].as_str().unwrap();

    Ok(access_token.to_string())
}

async fn share_folders(
    access_token: &str,
    folder_paths: &Vec<String>,
    emails: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://api.dropboxapi.com/2/sharing/share_folder_batch";

    let entries = folder_paths
        .iter()
        .map(|path| {
            json!({
                "path": path,
                "members": emails.iter().map(|email| {
                    json!({
                        "member": {
                            ".tag": "email",
                            "email": email
                        },
                        "access_level": {
                            ".tag": "editor"
                        }
                    })
                }).collect::<Vec<Value>>()
            })
        })
        .collect::<Vec<Value>>();

    let request_body = json!({ "entries": entries });

    let client = reqwest::Client::new();

    let response = client
        .post(url)
        .bearer_auth(access_token)
        .json(&request_body)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Los grupos de carpetas se compartieron exitosamente");
    } else {
        let error_message = response.text().await?;
        println!(
            "Error al compartir los grupos de carpetas: {}",
            error_message
        );
    }

    Ok(())
}

async fn get_folder_list(access_token: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let url = "https://api.dropboxapi.com/2/files/list_folder";

    let request_body = json!({
        "path": "",
        "recursive": false,
        "include_media_info": false,
        "include_deleted": false,
        "include_has_explicit_shared_members": false,
        "include_mounted_folders": true
    });

    let client = reqwest::Client::new();

    let response = client
        .post(url)
        .bearer_auth(access_token)
        .json(&request_body)
        .send()
        .await?;

    if response.status().is_success() {
        let response_data: Value = response.json().await?;
        let entries = response_data["entries"].as_array().unwrap();

        let folder_paths: Vec<String> = entries
            .iter()
            .filter(|entry| entry["path_display"].is_string())
            .map(|entry| entry["path_display"].as_str().unwrap().to_string())
            .collect();

        Ok(folder_paths)
    } else {
        let error_message = response.text().await?;
        println!("Error al obtener la lista de carpetas: {}", error_message);
        Err("Error al obtener la lista de carpetas".into())
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let credentials = get_credentials()?;
    let access_token = get_access_token(&credentials).await?;

    let folder_paths = get_folder_list(&access_token).await?;
    let emails = vec!["email@email.com"];

    share_folders(&access_token, &folder_paths, &emails).await?;
    

    Ok(())
}
