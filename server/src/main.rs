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


