#![feature(collections)]
#[inline(always)]
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
extern crate websocket;

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
use websocket::{Server, Message, Sender, Receiver};
use websocket::header::WebSocketProtocol;
use std::collections::HashMap;


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
struct InboundRecord{
    record_type:String,
    amount:f64
}

#[derive(RustcEncodable)]
struct OutboundRecord{
    id: i32,
    record_type:String,
    amount:f64,
    createdAt:String
}

#[derive(RustcDecodable)]
struct InboundTransaction{
    external_transaction_id:String,
    transaction_code:String,
    transaction_type:String,
    external_account_id:String,
    product_type:String,
    transaction_date:String,
    transaction_amount:Option<String>,
    debit_credit:String,
    business_personal:String,
    domestic_international:String,
    risk_rating:Option<String>,
    customer_industry_type:String
}


struct InboundUpload{
    id: i32,
    name:String,
    createdAt:chrono::NaiveDateTime
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

pub struct SocketsWrapper;

impl Key for SocketsWrapper{
    //type Value=Arc<Mutex<Vec<websocket::server::sender::Sender<websocket::stream::WebSocketStream>>>>;
    type Value=Arc<Mutex<HashMap<&'static str,websocket::server::sender::Sender<websocket::stream::WebSocketStream>>>>;
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

fn upload_records(request: &mut Request) -> IronResult<Response> {
    //thread::sleep_ms(5000);
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
        let record: InboundRecord = record.unwrap();
        let statement=connection.prepare("INSERT INTO records (record_type,amount) VALUES ($1,$2) RETURNING *").unwrap();
        let query_result=statement.query(&[&record.record_type,&record.amount]).unwrap();
    }
    //println!("{:?}",final_csv);

    Ok(Response::with((status::Ok, json::encode(&payload).unwrap())))
    //Ok(Response::with((status::Ok, "{\"client\":{\"id\":\"46\",\"name\":\"zzz\"}}")))
}

fn upload_records_inserts(request: &mut Request) -> IronResult<Response> {
    let a=time::now();
    println!("{:?}",a);
    //thread::sleep_ms(5000);
    let re = Regex::new(r"((?s)Content-Type: text/csv\r\n\r\n.*?\n\r)").unwrap();
    let new_re=Regex::new(r"(\s)+").unwrap();
    let mut payload = String::new();
    request.body.read_to_string(&mut payload).unwrap();
    let form_match=re.captures(&payload).unwrap().at(1).unwrap_or("").replace("Content-Type: text/csv","\n");
    let final_csv=Regex::new(r"\s{2,}").unwrap().replace_all(&form_match,"");
    let mut new_csv_rdr = csv::Reader::from_string(final_csv);
    let mutex = request.get::<persistent::Read<PostgresWrapper>>().unwrap();
    let connection=mutex.lock().unwrap();
    let mut insert_list=vec![];
    for record in new_csv_rdr.decode() {
        let record: InboundRecord = record.unwrap();
        //let statement=connection.prepare("INSERT INTO records (record_type,amount) VALUES ($1,$2) RETURNING *").unwrap();
        //let query_result=statement.query(&[&record.record_type,&record.amount]).unwrap();
        insert_list.push(format!("('{}',  '{}')",record.record_type,record.amount));
        if insert_list.len()>9999{
            connection.execute(&format!("INSERT INTO records (record_type,amount) VALUES {}",insert_list.connect(", ")),&[]).unwrap();
            insert_list.clear();
        }
    }
    //println!("{:?}",final_csv);
    connection.execute(&format!("INSERT INTO records (record_type,amount) VALUES {}",insert_list.connect(", ")),&[]).unwrap();
    println!("{:?}",insert_list.len());
    let b=time::now();
    println!("{:?}",b-a);
    Ok(Response::with((status::Ok, json::encode(&payload).unwrap())))
    //Ok(Response::with((status::Ok, "{\"client\":{\"id\":\"46\",\"name\":\"zzz\"}}")))
}

fn upload_transactions_inserts(request: &mut Request) -> IronResult<Response> {
    let a=time::now();
    let mut sockets_vector = request.get::<persistent::Read<SocketsWrapper>>().unwrap();
    let mut socket=sockets_vector.lock().unwrap();
    let mut sender=socket.get_mut("hello").unwrap();
    //thread::sleep_ms(5000);
    //let re = Regex::new(r"((?s)Content-Type: text/csv\r\n\r\n.*?\n\r)").unwrap();
    //let new_re=Regex::new(r"(\s)+").unwrap();
    //let space_re=Regex::new(r"\n\r\n").unwrap();
    let mut payload = String::new();
    request.body.read_to_string(&mut payload).unwrap();
    let b=time::now();
    println!("{:?}",b-a);
    //let form_match=re.captures(&payload).unwrap().at(1).unwrap_or("").replace("Content-Type: text/csv","\n");
    let c=time::now();
    println!("{:?}",c-a);
    //let final_csv=space_re.split(&payload).collect::<Vec<&str>>()[1];
    let beg_find=payload.find("\n\r\n").unwrap();
    let end_find=payload.rfind("\n\r\n").unwrap();
    let final_csv=payload.slice_chars(beg_find+3,end_find);
    let d=time::now();
    println!("{:?}",d-a);
    let mut new_csv_rdr = csv::Reader::from_string(final_csv);
    let connection =Connection::connect("postgres://kempchee:kempchee@localhost/rust_test", &SslMode::None).unwrap();
    let mut insert_list=vec![];
    let e=time::now();
    println!("{:?}",e-a);
    let mut finish_insert=true;
    let date_regex=Regex::new(r"[0-9]{1,2}/[0-9]{1,2}/[0-9]{4}").unwrap();
    let rows = new_csv_rdr.decode().collect::<csv::Result<Vec<InboundTransaction>>>().unwrap();
    let mut count = rows.len();
    let mut rows_length=count as f64;
    let mut current_index:f64=0.0;
    let null_value="NULL".to_string();
    for transaction in rows.iter() {
        //let transaction:InboundTransaction=transaction.unwrap();
        if date_regex.is_match(&transaction.transaction_date){}else{
            finish_insert=false;
            break;
        }
        let date_vec=transaction.transaction_date.split("/").map(|x|x.parse::<u32>().ok().unwrap()).collect::<Vec<u32>>();
        let date=chrono::naive::date::NaiveDate::from_ymd(date_vec[2] as i32,date_vec[0],date_vec[1]);
        let ref risk_rating=if transaction.risk_rating.is_some(){
            transaction.risk_rating.as_ref().unwrap()
        }else{
            &null_value
        };
        let ref transaction_amount=if transaction.transaction_amount.is_some(){
            transaction.transaction_amount.as_ref().unwrap()
        }else{
            &null_value
        };
        insert_list.push(format!("('{}', '{}', '{}', '{}', '{}', '{}', {}, '{}', '{}', '{}', {}, '{}')",
            transaction.external_transaction_id,
            transaction.transaction_code,
            transaction.transaction_type,
            transaction.external_account_id,
            transaction.product_type,
            date,
            transaction_amount,
            transaction.debit_credit,
            transaction.business_personal,
            transaction.domestic_international,
            risk_rating,
            transaction.customer_industry_type
        ));
        if insert_list.len()>50000{
            connection.execute(&format!("INSERT INTO transactions (external_transaction_id,transaction_code,transaction_type,external_account_id,product_type,transaction_date,transaction_amount,debit_credit,business_personal,domestic_international,risk_rating,customer_industry_type) VALUES {}",insert_list.connect(", ")),&[]).unwrap();
            insert_list.clear();
            current_index+=50000.0;
            sender.send_message(websocket::Message::Text("{\"message\":\"transactions_created\",\"upload_status\":".to_string()+&(((current_index/rows_length)*100.0).to_string())+"}")).unwrap();
        }
    }
    //println!("{:?}",final_csv);
    //println!("{:?}",insert_list);
    if finish_insert{
        connection.execute(&format!("INSERT INTO transactions (external_transaction_id,transaction_code,transaction_type,external_account_id,product_type,transaction_date,transaction_amount,debit_credit,business_personal,domestic_international,risk_rating,customer_industry_type) VALUES {}",insert_list.connect(", ")),&[]).unwrap();
        sender.send_message(websocket::Message::Text("{\"message\":\"transactions_created\",\"upload_status\":100}".to_string())).unwrap();
        let f=time::now();
        println!("{:?}",f-a);
        Ok(Response::with((status::Ok, "{\"message\":\"Your upload was successful!\"}")))
    }else{
        Ok(Response::with((status::Status::InternalServerError, "{\"message\":\"Your upload contained malformed data!\"}")))
    }
    //Ok(Response::with((status::Ok, "{\"client\":{\"id\":\"46\",\"name\":\"zzz\"}}")))
}

fn upload_transactions_experimental(request: &mut Request) -> IronResult<Response> {
    //let mut sockets_vector = request.get::<persistent::Read<SocketsWrapper>>().unwrap();
    //let mut socket=sockets_vector.lock().unwrap();
    //let mut sender=socket.get_mut("hello").unwrap();
    //sender.send_message(websocket::Message::Text("{\"message\":\"Nothing much yet!\"}".to_string())).unwrap();
    let params=HashMap::<String,String>::new();
    let mut payload = String::new();
    request.body.read_to_string(&mut payload).unwrap();
    let boundary_regex=Regex::new(r"(boundary=.*$)").unwrap();
    let mut new_vector=request.headers.get_raw("content-type").unwrap();
    let content_type=std::str::from_utf8(&new_vector[0][..]).unwrap();
    let mut boundary_capture=boundary_regex.captures(content_type).unwrap().at(1).unwrap_or("");
    let mut boundary=boundary_capture.slice_chars(13,boundary_capture.len());
    let separator_regex_string="((?s)".to_string()+&boundary+".*?"+"------)";
    println!("{:?}",separator_regex_string);
    let params_regex=Regex::new(&separator_regex_string).unwrap();
    for form_part in params_regex.captures_iter(&payload){
        let form_part_text=form_part.at(1).unwrap_or("");
        println!("{:?}",form_part_text);
        let name_string=Regex::new(r"(name=.*?[\s;])").unwrap().captures(form_part_text).unwrap().at(1).unwrap_or("");
        let name_value_string=name_string.slice_chars(6,name_string.len()-2);
        println!("{:?}",name_string);
        println!("{:?}",name_value_string);
    }
    println!("{:?}",boundary);
    println!("{:?}",payload);
    println!("{:?}",time::precise_time_ns());
    Ok(Response::with((status::Ok, "{\"message\":\"Your upload was successful!\"}")))
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
    let sockets_vector = request.get::<persistent::Read<SocketsWrapper>>().unwrap();
    println!("{:?}",sockets_vector.lock().unwrap().len());
    let connection=mutex.lock().unwrap();
    let statement=connection.prepare("SELECT * from client").unwrap();
    let mut clients_string="{\"clients\":[".to_string();
    let query_result=statement.query(&[]).unwrap();
    for (i,row) in query_result.iter().enumerate(){
        let client=Client{
            id:row.get(0),
            name:row.get(1)
        };
        if i!=query_result.iter().collect::<Vec<postgres::rows::Row>>().len()-1{
            clients_string=clients_string+&json::encode(&client).unwrap()+",";
        }else{
            clients_string=clients_string+&json::encode(&client).unwrap();
        }
        //println!("{}",json::encode(&client).unwrap());
        //println!("{}",i);
    }
    clients_string=clients_string+"]}";
    //println!("{}",clients_string);
    Ok(Response::with((status::Ok, clients_string)))

}


fn main() {
    let conn = Arc::new(Mutex::new(Connection::connect("postgres://kempchee:kempchee@localhost/rust_test", &SslMode::None).unwrap()));
    let mut router=Router::new();
    let original_connections=Arc::new(Mutex::new(HashMap::<&'static str,websocket::server::sender::Sender<websocket::stream::WebSocketStream>>::new()));
    let socket_connections=original_connections.clone();
    thread::spawn(move||{
        let server = Server::bind("localhost:4000").unwrap();

    	for connection in server {
            let socket_connections = socket_connections.clone();
    		// Spawn a new thread for each connection.
    		thread::spawn(move ||{
    			let request = connection.unwrap().read_request().unwrap(); // Get the request
    			let headers = request.headers.clone(); // Keep the headers so we can check them

    			request.validate().unwrap(); // Validate the request

    			let mut response = request.accept(); // Form a response

    			if let Some(&WebSocketProtocol(ref protocols)) = headers.get() {
    				if protocols.contains(&("rust-websocket".to_string())) {
    					// We have a protocol we want to use
    					response.headers.set(WebSocketProtocol(vec!["rust-websocket".to_string()]));
    				}
    			}

    			let mut client = response.send().unwrap(); // Send the response

    			let ip = client.get_mut_sender()
    				.get_mut()
    				.peer_addr()
    				.unwrap();

    			println!("Connection from {}", ip);
                let socket_id=time::precise_time_ns();
                client.send_message(websocket::Message::Text("{\"new_socket_id\":".to_string()+&socket_id.to_string()+"}")).unwrap();
    			let (mut sender, mut receiver) = client.split();
                {
                    let mut socket_connections = socket_connections.lock().unwrap();
                    socket_connections.insert("hello",sender);
                }
                for message in receiver.incoming_messages() {
    				let message = message.unwrap();

    				match message {
    					Message::Close(_) => {
    						let message = Message::Close(None);
    						//sender.send_message(message).unwrap();
    						println!("Client {} disconnected", ip);
    						return;
    					}
    					Message::Ping(data) => {
    						let message = Message::Pong(data);
    						//sender.send_message(message).unwrap();
    					}
    					_ => ()
    				}
			      }

                //println!("{:?}",socket_connections.unwrap());

    		});
    	}
    });
    router.get("/", hello_world);
    router.get("/clients",clients_index);
    router.post("/clients", create_client);
    router.post("/upload_records",upload_records);
    router.post("/upload_records_inserts",upload_records_inserts);
    router.post("/upload_transactions_inserts",upload_transactions_inserts);
    router.post("/upload_transactions_experimental",upload_transactions_experimental);
    let mut message_chain = Chain::new(router);
    message_chain.link_after(ResponseTime);
    message_chain.link(persistent::Read::<PostgresWrapper>::both(conn));
    message_chain.link(persistent::Read::<SocketsWrapper>::both(original_connections));

    Iron::new(message_chain).http("localhost:3000").unwrap();
    println!("On 3000");
}
