use warp::Filter;
use warp::ws::{Message, WebSocket};
use tokio::sync::mpsc;
use tonic::{transport::Server, Request, Response, Status};
use futures::StreamExt;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use std::fs::File;
use std::io::Write;
use uuid::Uuid;

// Include the generated protobuf code
tonic::include_proto!("media");

// Import the generated server types
use crate::media_service_server::{MediaService, MediaServiceServer};

// Import models and schema
use crate::models::{MediaFile, NewMediaFile};
use crate::schema::media_files::dsl::media_files;

#[path = "../models.rs"]
mod models;

#[path = "../schema.rs"]
mod schema;

#[derive(Clone)]
struct MediaServiceImpl {
    db: SqliteConnection,
}

#[tonic::async_trait]
impl MediaService for MediaServiceImpl {
    async fn upload_media(
        &self,
        request: Request<tonic::Streaming<MediaChunk>>,
    ) -> Result<Response<UploadResponse>, Status> {
        let mut stream = request.into_inner();
        let file_id = Uuid::new_v4().to_string();
        let mut file_name = String::new();
        let mut file_path = String::new();
        let mut file: Option<File> = None;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if file_name.is_empty() {
                file_name = chunk.name;
                file_path = format!("uploads/{}", file_id);
                file = Some(File::create(&file_path)?);
            }
            if let Some(f) = &mut file {
                f.write_all(&chunk.data)?;
            }
        }

        if let Some(mut f) = file {
            f.flush()?;
        }

        let new_file = NewMediaFile {
            name: file_name.clone(),
            path: file_path.clone(),
        };

        diesel::insert_into(media_files::table)
            .values(&new_file)
            .execute(&self.db)
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        Ok(Response::new(UploadResponse {
            file_id,
            message: format!("File {} uploaded successfully", file_name),
        }))
    }
}

async fn ws_handler(ws: WebSocket, db: SqliteConnection) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let _ = ws_tx.send(Message::text(msg)).await;
        }
    });

    while let Some(result) = ws_rx.next().await {
        if let Ok(msg) = result {
            if msg.is_text() && msg.to_str().unwrap() == "list_files" {
                let files: Vec<MediaFile> = media_files::table
                    .load(&db)
                    .expect("Error loading files");
                let files_json = serde_json::to_string(&files).unwrap();
                let _ = tx.send(files_json);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("uploads")?;
    let db = SqliteConnection::establish("sqlite://media.db")?;
    let media_service = MediaServiceImpl { db: db.clone() };

    let grpc = tokio::spawn(
        Server::builder()
            .add_service(MediaServiceServer::new(media_service))
            .serve("0.0.0.0:50051".parse()?),
    );

    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::any().map(move || db.clone()))
        .map(|ws: warp::ws::Ws, db| {
            ws.on_upgrade(move |websocket| ws_handler(websocket, db))
        });

    let warp = warp::serve(ws_route).run(([0, 0, 0, 0], 3030));

    tokio::select! {
        _ = grpc => println!("gRPC server exited"),
        _ = warp => println!("Warp server exited"),
    }

    Ok(())
}