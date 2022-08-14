use serde::{Deserialize, Serialize};

use azure_core::error::Result;
use azure_data_cosmos::prelude::*;
use clap::Parser;
use futures::stream::StreamExt;
use time::OffsetDateTime;

#[derive(Debug, Parser)]
struct Args {
    #[clap(env = "COSMOS_PRIMARY_KEY")]
    primary_key: String,
    #[clap(env = "COSMOS_ACCOUNT")]
    account: String,
    #[clap(env = "LOCATION")]
    location: String,
    #[clap(env = "DATABASE_NAME")]
    database_name: String,
    #[clap(env = "COLLECTION_NAME")]
    collection_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct MySampleStruct {
    id: String,
    number: u64,
    timestamp: i64,
}

impl azure_data_cosmos::CosmosEntity for MySampleStruct {
    type Entity = u64;

    fn partition_key(&self) -> Self::Entity {
        self.number
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let primary_key = std::env::var("COSMOS_PRIMARY_KEY").expect("Set env variables COSMOS_PRIMARY_KEY first!");
    let account = std::env::var("COSMOS_ACCOUNT").expect("Set env variable COSMOS_ACCOUNT first!");
    
    let args = Args::parse();
    let authorization_token = AuthorizationToken::primary_from_base64(&args.primary_key)?;
    let client = CosmosClient::new(account.clone(), authorization_token);
    let database = client.database_client(args.database_name);
    let collection = database.collection_client(args.collection_name);

    println!("Inserting 10 documents...");
    let mut session_token = None;
    for i in 0..10 {
        let document_to_insert = MySampleStruct {
            id: format!("unique_id{i}"),
            number: i * 100,
            timestamp: OffsetDateTime::now_utc().unix_timestamp(),
        };

        session_token = Some(
            collection
                .create_document(document_to_insert)
                .is_upsert(true)
                .into_future()
                .await?
                .session_token,
        );
    }
    println!("Done!");

    let session_token = ConsistencyLevel::Session(session_token.unwrap());
    
    println!("\nStreaming documents");
    let mut stream = collection
        .list_documents()
        .consistency_level(session_token.clone())
        .max_item_count(3)
        .into_stream::<MySampleStruct>();
        while let Some(res) = stream.next().await {
        let res = res?;
        println!("received {} documents in one batch!", res.documents.len());
        res.documents
            .iter()
            .for_each(|doc| println!("Document: {:#?}", doc));
    };

    println!("\nQuerying documents");
    let query_documents_response = collection
        .query_documents("SELECT * FROM A WHERE A.number < 600")
        .query_cross_partition(true)
        .consistency_level(session_token)
        .into_stream::<MySampleStruct>()
        .next()
        .await
        .unwrap()?;

    println!(
        "Received {} documents!",
        query_documents_response.results.len()
    );

    query_documents_response.documents().for_each(|document| {
        println!("number ==> {}", document.number);
    });

    let session_token = ConsistencyLevel::Session(query_documents_response.session_token);
    for (document, document_attributes) in query_documents_response.results {
        println!(
            "deleting id == {}, a_number == {}.",
            document.id, document.number
        );

        collection
            .document_client(document.id, &document.number)?
            .delete_document()
            .consistency_level(session_token.clone())
            .if_match_condition(&document_attributes.unwrap())
            .into_future()
            .await?;
    }

    let list_documents_response = collection
        .list_documents()
        .consistency_level(session_token)
        .into_stream::<serde_json::Value>()
        .next()
        .await
        .unwrap()?;
    assert_eq!(list_documents_response.documents.len(), 4);

    Ok(())
}