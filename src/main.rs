use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::{error, web, App, Error, HttpResponse, HttpServer};
use futures_util::stream::StreamExt as _;
use std::fs;
use std::io::Write;
use uuid::Uuid;

use lettre::message::Message;
use lettre::{SmtpTransport, Transport};

#[derive(serde::Deserialize)]
struct UploadRequest {
    email: String,
}

async fn index() -> Result<NamedFile, Error> {
    Ok(NamedFile::open("./public/index.html")?)
}

async fn upload(mut payload: Multipart) -> Result<HttpResponse, Error> {
    let upload_path = "./uploads";
    fs::create_dir_all(upload_path)?;

    let mut email = None;
    let mut saved_file_path = None;

    while let Some(item) = payload.next().await {
        let mut field = item?;
        let content_disposition = field.content_disposition().unwrap();
        let name = content_disposition.get_name().unwrap();

        if name == "file" {
            let filename = format!("upload-{}", Uuid::new_v4());
            let filepath = format!("{}/{}", upload_path, filename);

            let mut f = fs::File::create(&filepath)?;

            while let Some(chunk) = field.next().await {
                let data = chunk?;
                f.write_all(&data)?;
            }

            saved_file_path = Some(filepath);
        } else if name == "email" {
            let mut bytes = web::BytesMut::new();

            while let Some(chunk) = field.next().await {
                bytes.extend_from_slice(&chunk?);
            }

            email = Some(String::from_utf8(bytes.to_vec()).unwrap());
        }
    }

    let email = match email {
        Some(e) => e,
        None => return Ok(HttpResponse::BadRequest().body("Email non fourni")),
    };

    let filepath = match saved_file_path {
        Some(p) => p,
        None => return Ok(HttpResponse::BadRequest().body("Fichier non fourni")),
    };

    let filename = filepath
        .split('/')
        .last()
        .unwrap_or("unknown");

    let download_url = format!("http://localhost:8080/download/{}", filename);

    if let Err(e) = send_email(email.clone(), download_url.clone()).await {
        return Ok(HttpResponse::InternalServerError().body(format!(
            "Erreur lors de l'envoi d'email: {}",
            e
        )));
    }

    Ok(HttpResponse::Ok().body(format!(
        "Fichier reçu ! Téléchargez-le ici : {}",
        download_url
    )))
}

async fn send_email(email: String, link: String) -> Result<(), Box<dyn std::error::Error>> {
    let email = Message::builder()
        .from("WebTransfer <webtransfer@test.lan>".parse()?) // Doit correspondre à `myhostname`
        .to(email.parse()?)
        .subject("Votre fichier est prêt à être téléchargé")
        .body(format!("Voici votre lien de téléchargement : {}", link))?;

    let mailer = SmtpTransport::builder_dangerous("127.0.0.1")
        .port(25)
        .build();

    match mailer.send(&email) {
        Ok(_) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}

async fn download(path: web::Path<String>) -> Result<NamedFile, Error> {
    let filename = path.into_inner();
    let full_path = format!("./uploads/{}", filename);

    let file = NamedFile::open(full_path).map_err(|_| error::ErrorNotFound("Fichier non trouvé"))?;
    Ok(file)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 Serveur démarré sur http://localhost:8080");

    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .route("/upload", web::post().to(upload))
            .route("/download/{filename}", web::get().to(download))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
