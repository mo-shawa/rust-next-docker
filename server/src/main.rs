use postgres::{Client, NoTls};
use postgres::Error as PostgresError;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::env;

#[macro_use]
extern crate serde_derive;

#[derive(Serialize, Deserialize)]
struct User {
    id: Option<i32>,
    name: String,
    email: String,
}

const DB_URL: &str = env!("DATABASE_URL");

// Constants
const OK_RESPONSE: &str =
    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, PUT, DELETE\r\nAccess-Control-Allow-Headers: Content-Type\r\n\r\n";

const NOT_FOUND_RESPONSE: &str =
    "HTTP/1.1 404 NOT FOUND\r\n\r\n";

const INTERNAL_SERVER_ERROR_RESPONSE: &str =
    "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";


fn main(){
    if let Err(_) = set_database(){
        println!("Error setting up database");
        return;
    }

    let listener = TcpListener::bind(format!("0.0.0.0:8080")).unwrap();
    println!("Server listening on port 8080");

    for stream in listener.incoming(){
        match stream {
            Ok(stream) => {
                handle_client(stream);
            }
            Err(e) => {
                println!("Unable to connect: {}", e);
            }
        }
    }
}

fn set_database()  -> Result<(), PostgresError>{
    let mut client = Client::connect(DB_URL, NoTls)?;

    client.batch_execute("
        CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL UNIQUE,
            email VARCHAR NOT NULL UNIQUE,
    ")?;
    Ok(())
}

fn get_id(request: &str) -> &str {
    request.split("/").nth(4).unwrap_or_default().split_whitespace().next().unwrap_or_default()
}

fn get_user_request_body(request: &str) -> Result<User, serde_json::Error> {
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}

// handle requests
fn handle_client(mut stream: TcpStream){
    let mut buffer = [0; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer){
        Ok(size) => {
            request.push_str(String::from_utf8_lossy(&buffer).as_ref());

            let (status_line, content) = match &*request {
                r if r.starts_with("OPTIONS") => (OK_RESPONSE.to_string(), "".to_string()),
                r if r.starts_with("POST /api/rust/users" )=> handle_post_request(r),
                r if r.starts_with("GET /api/rust/users") => handle_get_request(r),
                r if r.starts_with("GET /api/rust/users" )=> handle_get_all_request(r),
                r if r.starts_with("PUT /api/rust/users") => handle_put_request(r),
                r if r.starts_with("DELETE /api/rust/users") => handle_delete_request(r),
                _ => (NOT_FOUND_RESPONSE.to_string(), "404 not found".to_string()),
            };
            
            return stream.write_all(format!("{}{}", status_line, content).as_bytes()).unwrap();
        }
        Err(e) => return eprintln!("Unable to read stream: {}", e),
    }
}

fn handle_post_request(request: &str) -> (String, String) {
    match (get_user_request_body(request), Client::connect(DB_URL, NoTls)) {
        (Ok(user), Ok(mut client)) => {
            let row = client.query_one(
                "
                INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id
                ", &[&user.name, &user.email]).unwrap();
            let user_id: i32 = row.get(0);
            
            match client.query_one("
                SELECT id, name, email FROM users WHERE id = $1
            ", &[&user_id]){
                Ok(row) => {
                    let user = User {
                        id: row.get(0),
                        name: row.get(1),
                        email: row.get(2),
                    };
                   return (OK_RESPONSE.to_string(), serde_json::to_string(&user).unwrap())
                }
                Err(e) => return (INTERNAL_SERVER_ERROR_RESPONSE.to_string(), "Failed to retrieve created user".to_string())
                }

            }
            _ => return (INTERNAL_SERVER_ERROR_RESPONSE.to_string(), "Internal server error".to_string()),
        }
    }

fn handle_get_request(request: &str) -> (String, String) {
    match (get_id(&request).parse::<i32>(), Client::connect(DB_URL, NoTls)){
            (Ok(user_id), Ok(mut client)) => {
                match client.query_one("SELECT * FROM users WHERE id = $1", &[&user_id]){
                    Ok(row) => {
                        let user = User {
                            id: row.get(0),
                            name: row.get(1),
                            email: row.get(2),
                        };
                        return (OK_RESPONSE.to_string(), serde_json::to_string(&user).unwrap())
                        }
                        _ => return (NOT_FOUND_RESPONSE.to_string(), "No users found".to_string()),
                    }
                }
                _ => return (INTERNAL_SERVER_ERROR_RESPONSE.to_string(), "Internal server error".to_string()),
            }
    }

fn handle_get_all_request(request: &str) -> (String, String){
    match Client::connect(DB_URL, NoTls){
        Ok(mut client) => {
            let mut users: Vec<User> = Vec::new();
            for row in client.query("SELECT * FROM users", &[]).unwrap() {
                users.push(User {
                    id: row.get(0),
                    name: row.get(1),
                    email: row.get(2),
                });
            };
            return (OK_RESPONSE.to_string(), serde_json::to_string(&users).unwrap())
        }
        _ => return (INTERNAL_SERVER_ERROR_RESPONSE.to_string(), "Error fetching users".to_string())
    }
}

fn handle_put_request(request: &str) -> (String, String) {
    match(
        get_id(&request).parse::<i32>(), 
        get_user_request_body(request),
        Client::connect(DB_URL, NoTls)
    ){
        (Ok(id), Ok(user), Ok(mut client)) => {
            client.execute("UPDATE users SET name = $1, email = $2 WHERE id = $3", &[&user.name, &user.email, &id])
            .unwrap();

            return (OK_RESPONSE.to_string(), "User updated".to_string())
        }
       _ => return (INTERNAL_SERVER_ERROR_RESPONSE.to_string(), "Internal error".to_string())
    }
}



