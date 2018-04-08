use std::sync::{Arc, Mutex};
use std::io::Read;
use iron::{status, AfterMiddleware, Handler, IronResult, Request, Response};
use iron::headers::ContentType;
use rustc_serialize::json;

use database::Database;
use uuid::Uuid;
use router::Router;
use models::Post;
use std::error::Error;

macro_rules! try_handler {
    ($e:expr) => {
        match $e {
            Ok(x) => x,
            Err(e) => return Ok(Response::with((status::InternalServerError, e.description()))),
        }
    };
    ($e:expr, $error:expr) => {
        match $e {
            Ok(x) => x,
            Err(e) => return Ok(Response::with(($error, e.description()))),
        }
    };
}

macro_rules! lock {
    ($e:expr) => {$e.lock().unwrap()}
}

macro_rules! get_http_params {
    ($r:expr, $e:expr) => {
        match $r.extensions.get::<Router>() {
            Some(router) => {
                match router.find($e) {
                    Some(v) => v,
                    None => return Ok(Response::with(status::BadRequest)),
                }
            },
            None => return Ok(Response::with(status::InternalServerError)),
        }
    }
}

pub struct Handlers {
    pub list_post: ListPost,
    pub create_post: CreatePost,
    pub show_post: ShowPost,
}

impl Handlers {
    pub fn new(db: Database) -> Handlers {
        let database = Arc::new(Mutex::new(db));

        Handlers {
            list_post: ListPost::new(database.clone()),
            create_post: CreatePost::new(database.clone()),
            show_post: ShowPost::new(database.clone()),
        }
    }
}

pub struct ListPost {
    database: Arc<Mutex<Database>>,
}

impl ListPost {
    fn new(database: Arc<Mutex<Database>>) -> ListPost {
        ListPost { database }
    }
}

impl Handler for ListPost {
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        let payload = try_handler!(json::encode(lock!(self.database).posts()));
        Ok(Response::with((status::Ok, payload)))
    }
}

pub struct CreatePost {
    database: Arc<Mutex<Database>>,
}

impl CreatePost {
    fn new(database: Arc<Mutex<Database>>) -> CreatePost {
        CreatePost { database }
    }
}

impl Handler for CreatePost {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let mut payload = String::new();
        try_handler!(req.body.read_to_string(&mut payload));

        let post = try_handler!(json::decode(&payload), status::BadRequest);

        lock!(self.database).add_post(post);
        Ok(Response::with((status::Created, payload)))
    }
}

pub struct ShowPost {
    database: Arc<Mutex<Database>>,
}

impl ShowPost {
    fn new(database: Arc<Mutex<Database>>) -> ShowPost {
        ShowPost { database }
    }

    fn find_post(&self, id: &Uuid) -> Option<Post> {
        let locked = lock!(self.database);
        let mut iterator = locked.posts().iter();
        iterator.find(|p| p.uuid() == id).map(|p| p.clone())
    }
}

impl Handler for ShowPost {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let ref post_id = get_http_params!(req, "id");
        let id = try_handler!(Uuid::parse_str(post_id), status::BadRequest);

        if let Some(post) = self.find_post(&id) {
            let payload = try_handler!(json::encode(&post), status::InternalServerError);
            Ok(Response::with((status::Ok, payload)))
        } else {
            Ok(Response::with((status::NotFound)))
        }
    }
}

pub struct JsonAfterMiddleware;

impl AfterMiddleware for JsonAfterMiddleware {
    fn after(&self, _: &mut Request, mut res: Response) -> IronResult<Response> {
        res.headers.set(ContentType::json());
        Ok(res)
    }
}