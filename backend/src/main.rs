use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_files::{Files, NamedFile};


async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

/*
async fn root() -> impl Responder {
    Files::new("/", "./frontend").index_file("index.html")
}
*/



#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/hello", web::get().to(hello))
            .service(Files::new("/", "./frontend").index_file("index.html"))
            //.service(Files::new("/pkg", "./frontend/pkg"))
            //.default_service(web::get().to(index))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
