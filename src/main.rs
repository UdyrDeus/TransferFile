use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::{error, web, App, Error, HttpResponse, HttpServer};
use futures_util::stream::StreamExt as _;
use std::fs;
use std::io::Write;
use uuid::Uuid;  // Assurez-vous d'importer `Uuid`

use lettre::message::Message; // Suppression de Mailbox inutilisé
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport, Transport}; // Suppression de `Tokio1Executor` ici

// Structure pour recevoir les données de la requête
#[derive(serde::Deserialize)]
struct UploadRequest {
    email: String,  // Champ pour l'email du destinataire
}

// Point d'entrée pour la page d'accueil
async fn index() -> Result<NamedFile, Error> {
    Ok(NamedFile::open("./public/index.html")?)
}

// Fonction pour traiter les uploads de fichiers
async fn upload(
    mut payload: Multipart,
    form: web::Json<UploadRequest>,
) -> Result<HttpResponse, Error> {
    let upload_path = "./uploads";
    fs::create_dir_all(upload_path)?; // Crée le dossier si nécessaire

    while let Some(item) = payload.next().await {
        let mut field = item?;

        // Utilisation de `Uuid::new_v4()` pour générer un UUID v4
        let filename = format!("upload-{}", Uuid::new_v4().to_string());  // Correction ici
        let filepath = format!("{}/{}", upload_path, filename);

        let mut f = fs::File::create(&filepath)?; // Crée le fichier

        // Écrit les données dans le fichier
        while let Some(chunk) = field.next().await {
            let data = chunk?;
            f.write_all(&data)?; // Écrit chaque morceau de données dans le fichier
        }

        let download_url = format!("http://localhost:8080/download/{}", filename);

        // Envoie l'e-mail avec le lien de téléchargement et l'email du destinataire
        if let Err(e) = send_email(form.email.clone(), download_url.clone()).await {
            return Ok(HttpResponse::InternalServerError().body(format!(
                "Erreur lors de l'envoi d'email: {}",
                e
            )));
        }

        return Ok(HttpResponse::Ok().body(format!(
            "Fichier reçu ! Téléchargez-le ici : {}",
            download_url
        )));
    }

    Ok(HttpResponse::BadRequest().body("Erreur d'upload"))
}

// Fonction pour envoyer l'email avec le lien de téléchargement
async fn send_email(email: String, link: String) -> Result<(), Box<dyn std::error::Error>> {
    let email = Message::builder()
        .from("WebTransfer <expediteur@example.com>".parse()?)
        .to(email.parse()?) // Utilisation de l'email du destinataire
        .subject("Votre fichier est prêt à être téléchargé")
        .body(format!("Voici votre lien de téléchargement : {}", link))?;

    let creds = Credentials::new("smtp_username".to_string(), "smtp_password".to_string());

    // Configuration du serveur SMTP
    let mailer = SmtpTransport::relay("smtp.example.com")? // Remplace par le serveur SMTP correct
        .credentials(creds)
        .build();

    // Tentative d'envoi de l'email
    match mailer.send(&email) {
        Ok(_) => Ok(()),
        Err(e) => Err(Box::new(e)), // Retourne l'erreur en cas d'échec
    }
}

// Fonction pour gérer le téléchargement des fichiers
async fn download(path: web::Path<String>) -> Result<NamedFile, Error> {
    let filename = path.into_inner();
    let full_path = format!("./uploads/{}", filename);

    let file = NamedFile::open(full_path).map_err(|_| error::ErrorNotFound("Fichier non trouvé"))?;
    Ok(file)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 Serveur démarré sur http://localhost:8080");

    // Démarre le serveur HTTP
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .route("/upload", web::post().to(upload))  // Requête POST pour l'upload avec email
            .route("/download/{filename}", web::get().to(download))
    })
    .bind(("127.0.0.1", 8080))? // Lie le serveur à l'adresse et au port
    .run()
    .await
}
