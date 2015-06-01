extern crate iron;
extern crate router;
extern crate hyper;
extern crate postgres;
extern crate rustc_serialize;
extern crate persistent;
extern crate unicase;

use postgres::{Connection, SslMode};
use iron::prelude::*;
use iron::status;
use router::Router;
use std::io::Read;
use iron::{AfterMiddleware,BeforeMiddleware,Headers};
use hyper::net::NetworkStream;
use hyper::buffer::BufReader;
use std::io;
use rustc_serialize::json;
use std::sync::{Arc, Mutex};
use std::thread;
use iron::typemap::Key;
use hyper::method::Method;
use unicase::UniCase;

#[derive(RustcDecodable, RustcEncodable)]
struct Client {
    id: i32,
    name: String
}

#[derive(RustcDecodable, RustcEncodable)]
struct ClientWrapper{
    client:Client
}

struct ResponseTime;

#[derive(RustcDecodable, RustcEncodable)]
struct ResponseMessage {
    message:String
}

pub struct PostgresWrapper;

impl Key for PostgresWrapper{
    type Value=Arc<Mutex<postgres::Connection>>;
}

impl AfterMiddleware for ResponseTime {
    fn after(&self, req: &mut Request, res: Response) -> IronResult<Response> {
        //res.headers.set(iron::headers::AccessControlAllowOrigin::Any);
        //println!("{:?}",res.headers);
        let mut new_headers= res.headers;
        new_headers.set(iron::headers::AccessControlAllowOrigin::Any);
        new_headers.set(iron::headers::AccessControlAllowMethods(vec![Method::Put,Method::Post,Method::Get,Method::Delete,Method::Options]));
        new_headers.set(iron::headers::Allow(vec![Method::Put,Method::Post,Method::Get,Method::Delete,Method::Options]));
        new_headers.set(iron::headers::AccessControlAllowHeaders(vec![UniCase("Content-Type".to_string()),UniCase("Origin".to_string()), UniCase("Accept".to_string()),UniCase("X-Requested-With".to_string())]));
        new_headers.set(iron::headers::AccessControlRequestHeaders(vec![UniCase("Content-Type".to_string()),UniCase("Origin".to_string()), UniCase("Accept".to_string()),UniCase("X-Requested-With".to_string())]));
        new_headers.set(iron::headers::ContentType(iron::mime::Mime(iron::mime::TopLevel::Application, iron::mime::SubLevel::Json,vec![])));
        //println!("{:?}",new_headers);
        let new_res=Response{
            status:res.status,
            body:res.body,
            headers:new_headers,
            extensions:res.extensions
        };
        Ok(new_res)
    }
}

fn hello_world(request: &mut Request) -> IronResult<Response> {
    let message = ResponseMessage{ message: "Hello, World".to_string() };
    let payload = json::encode(&message).unwrap();
    Ok(Response::with((status::Ok, payload)))
}

fn create_client(request: &mut Request) -> IronResult<Response> {
    //thread::sleep_ms(5000);
    let mut payload = String::new();
    request.body.read_to_string(&mut payload).unwrap();
    //let mut new_client_wrapper: ClientWrapper = json::decode(&payload).unwrap();
    println!("{}","hi");
    let mut new_client:Client=json::decode(&payload).unwrap();
    println!("boo");
    let mutex = request.get::<persistent::Read<PostgresWrapper>>().unwrap();
    let connection=mutex.lock().unwrap();
    let statement=connection.prepare("INSERT INTO client (name) VALUES ($1) RETURNING *").unwrap();
    let query_result=statement.query(&[&new_client.name]).unwrap();
    for client_row in query_result{
        let hello:i32=client_row.get(0);
        println!("{}",hello);
        new_client=Client{
            id:client_row.get(0),
            name:client_row.get(1)
        };
    }
    Ok(Response::with((status::Ok, json::encode(&new_client).unwrap())))
    //Ok(Response::with((status::Ok, "{\"client\":{\"id\":\"46\",\"name\":\"zzz\"}}")))
}

fn clients_index(request: &mut Request) -> IronResult<Response> {
    let mutex = request.get::<persistent::Read<PostgresWrapper>>().unwrap();
    let connection=mutex.lock().unwrap();
    let statement=connection.prepare("SELECT * from client").unwrap();
    let mut clients_string="{\"clients\":[".to_string();
    let query_result=statement.query(&[]).unwrap();
    for (i,row) in query_result.iter().enumerate(){
        let client=Client{
            id:row.get(0),
            name:row.get(1)
        };
        if i!=query_result.iter().collect::<Vec<postgres::Row>>().len()-1{
            clients_string=clients_string+&json::encode(&client).unwrap()+",";
        }else{
            clients_string=clients_string+&json::encode(&client).unwrap();
        }
        println!("{}",json::encode(&client).unwrap());
        println!("{}",i);
    }
    clients_string=clients_string+"]}";
    println!("{}",clients_string);
    Ok(Response::with((status::Ok, clients_string)))

}

fn main() {
    let conn = Arc::new(Mutex::new(Connection::connect("postgres://kempchee:kempchee@localhost/rust_test", &SslMode::None).unwrap()));
    let mut router=Router::new();

    router.get("/", hello_world);
    router.get("/clients",clients_index);
    router.post("/clients", create_client);
    let mut message_chain = Chain::new(router);
    message_chain.link_after(ResponseTime);
    message_chain.link(persistent::Read::<PostgresWrapper>::both(conn));

    Iron::new(message_chain).http("localhost:3000").unwrap();
    println!("On 3000");
}
