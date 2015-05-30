extern crate iron;
extern crate router;
extern crate hyper;
extern crate postgres;
extern crate rustc_serialize;
extern crate persistent;

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

#[derive(RustcDecodable, RustcEncodable)]
struct Client {
    name: String
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
        println!("{:?}",res.headers);
        let mut new_headers= res.headers;
        new_headers.set(iron::headers::AccessControlAllowOrigin::Any);
        println!("{:?}",new_headers);
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

    thread::sleep_ms(5000);
    let mut payload = String::new();
    request.body.read_to_string(&mut payload).unwrap();
    println!("{:?}",payload);
    let new_client: Client = json::decode(&payload).unwrap();
    let count = request.get::<persistent::Read<PostgresWrapper>>().unwrap();
    count.lock().unwrap().execute("INSERT INTO client (name) VALUES ($1)",
                 &[&new_client.name]).unwrap();
    Ok(Response::with((status::Ok, payload)))
}

fn main() {
    let conn = Arc::new(Mutex::new(Connection::connect("postgres://kempchee:kempchee@localhost/rust_test", &SslMode::None).unwrap()));
    let mut router=Router::new();

    router.get("/", hello_world);
    router.post("/create_client", create_client);
    let mut message_chain = Chain::new(router);
    message_chain.link_after(ResponseTime);
    message_chain.link(persistent::Read::<PostgresWrapper>::both(conn));

    Iron::new(message_chain).http("localhost:3000").unwrap();
    println!("On 3000");
}
