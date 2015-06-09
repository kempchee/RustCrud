#![feature(collections)]
extern crate iron;
extern crate router;
extern crate hyper;
extern crate postgres;
extern crate rustc_serialize;
extern crate persistent;
extern crate unicase;
extern crate csv;
extern crate chrono;
extern crate regex;
extern crate time;

use postgres::{Connection, SslMode};
use iron::prelude::*;
use iron::status;
use router::Router;
use std::io::Read;
use iron::{AfterMiddleware,BeforeMiddleware,Headers};
use hyper::net::NetworkStream;
use hyper::buffer::BufReader;
use std::io;
use rustc_serialize::json::{self, ToJson, Json};
use std::sync::{Arc, Mutex};
use std::thread;
use iron::typemap::Key;
use hyper::method::Method;
use unicase::UniCase;
use std::collections::BTreeMap;
use regex::Regex;
use chrono::naive::datetime::NaiveDateTime;
use std::fmt;


#[derive(RustcDecodable, RustcEncodable)]
struct Client {
    id: i32,
    name: String
}

#[derive(RustcDecodable, RustcEncodable)]
struct ClientWrapper{
    client:Client
}

#[derive(RustcDecodable, RustcEncodable)]
struct ClientsWrapper{
    clients:Client
}


struct MyDateTime{
    time:NaiveDateTime
}

impl fmt::Debug for MyDateTime{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.time)
    }
}

#[derive(RustcDecodable)]
struct Record{
    id: i32,
    recordType:String,
    amount:f64,
    createdAt:MyDateTime
}




impl rustc_serialize::Decodable for MyDateTime{
    fn decode<D: rustc_serialize::Decoder>(d: &mut D) -> Result<MyDateTime, D::Error> {
        Ok(MyDateTime{
            time:NaiveDateTime::from_timestamp(100,24)
            })
    }
}

struct Upload{
    id: i32,
    name:String,
    createdAt:chrono::NaiveDateTime
}

impl ToJson for Upload{
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        // All standard types implement `to_json()`, so use it
        d.insert("name".to_string(), self.name.to_json());
        d.insert("createdAt".to_string(), format!("{}",self.createdAt.format("%D")).to_json());
        Json::Object(d)
    }
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

fn upload_record(request: &mut Request) -> IronResult<Response> {
    //thread::sleep_ms(5000);
    //let webkit_unique=&(request.headers.get::<iron::headers::ContentType>().unwrap().0).2[0].1.to_string();
    //let mut search_string="((?is)--".to_string()+webkit_unique;
    //search_string=search_string.to_string()+".*?--";
    //search_string=search_string.to_string()+webkit_unique;
    //search_string=search_string.to_string()+")";
    //println!("{}",search_string);
    let re = Regex::new(r"((?s)Content-Type: text/csv\r\n\r\n.*?\n\r)").unwrap();
    let new_re=Regex::new(r"(\s)+").unwrap();
    let mut payload = String::new();
    request.body.read_to_string(&mut payload).unwrap();
    let form_match=re.captures(&payload).unwrap().at(1).unwrap_or("").replace("Content-Type: text/csv","\n");
    let final_csv=Regex::new(r"\s{2,}").unwrap().replace_all(&form_match,"");
    let mut new_csv_rdr = csv::Reader::from_string(final_csv);

    let mutex = request.get::<persistent::Read<PostgresWrapper>>().unwrap();
    let connection=mutex.lock().unwrap();
    for record in new_csv_rdr.decode() {
        let record: Record = record.unwrap();
        println!("({}, {},{},{:?})", record.recordType, record.amount,record.id, record.createdAt);
    }
    //println!("{:?}",final_csv);

    Ok(Response::with((status::Ok, json::encode(&payload).unwrap())))
    //Ok(Response::with((status::Ok, "{\"client\":{\"id\":\"46\",\"name\":\"zzz\"}}")))
}

fn create_client(request: &mut Request) -> IronResult<Response> {
    //thread::sleep_ms(5000);
    let mut payload = String::new();
    request.body.read_to_string(&mut payload).unwrap();
    let mut new_client_wrapper: ClientWrapper = json::decode(&payload).unwrap();
    println!("{}","hi");
    let mut new_client:Client=Client{id:0,name:"".to_string()};
    println!("boo");
    let mutex = request.get::<persistent::Read<PostgresWrapper>>().unwrap();
    let connection=mutex.lock().unwrap();
    let statement=connection.prepare("INSERT INTO client (name) VALUES ($1) RETURNING *").unwrap();
    let query_result=statement.query(&[&new_client_wrapper.client.name]).unwrap();
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
    router.post("/upload_record",upload_record);
    let mut message_chain = Chain::new(router);
    message_chain.link_after(ResponseTime);
    message_chain.link(persistent::Read::<PostgresWrapper>::both(conn));

    Iron::new(message_chain).http("localhost:3000").unwrap();
    println!("On 3000");
}
