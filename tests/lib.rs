use std::fs::File;

use cqrs_es::doc::{Customer, CustomerEvent};
use cqrs_es::persist::{PersistedEventStore, SemanticVersionEventUpcaster};
use cqrs_es::EventStore;
use sqlite_es::{default_sqlite_pool, SqliteEventRepository};
use serde_json::Value;
use sqlx::{Sqlite, Pool};

const TEST_CONNECTION_STRING: &str = "sqlite://test.db";

async fn create_new_db_and_pool() ->  Pool<Sqlite> {
    let _file = File::create("test.db");
    let pool = default_sqlite_pool(TEST_CONNECTION_STRING).await;
    let _result = sqlx::migrate!("./db").run(&pool).await;
    pool
}

async fn new_test_event_store(
    pool: Pool<Sqlite>,
) -> PersistedEventStore<SqliteEventRepository, Customer> {
    let repo = SqliteEventRepository::new(pool);
    PersistedEventStore::<SqliteEventRepository, Customer>::new_event_store(repo)
}

#[tokio::test]
async fn commit_and_load_events() {
    let pool = create_new_db_and_pool().await;
    let repo = SqliteEventRepository::new(pool);
    let event_store = PersistedEventStore::<SqliteEventRepository, Customer>::new_event_store(repo);

    simple_es_commit_and_load_test(event_store).await;
}

#[tokio::test]
async fn commit_and_load_events_snapshot_store() {
    let pool = create_new_db_and_pool().await;
    let repo = SqliteEventRepository::new(pool);
    let event_store =
        PersistedEventStore::<SqliteEventRepository, Customer>::new_aggregate_store(repo);

    simple_es_commit_and_load_test(event_store).await;
}

async fn simple_es_commit_and_load_test(
    event_store: PersistedEventStore<SqliteEventRepository, Customer>,
) {
    let id = uuid::Uuid::new_v4().to_string();
    assert_eq!(0, event_store.load_events(id.as_str()).await.unwrap().len());
    let context = event_store.load_aggregate(id.as_str()).await.unwrap();

    event_store
        .commit(
            vec![
                CustomerEvent::NameAdded {
                    name: "test_event_A".to_string(),
                },
                CustomerEvent::EmailUpdated {
                    new_email: "email A".to_string(),
                },
            ],
            context,
            Default::default(),
        )
        .await
        .unwrap();

    assert_eq!(2, event_store.load_events(id.as_str()).await.unwrap().len());
    let context = event_store.load_aggregate(id.as_str()).await.unwrap();

    event_store
        .commit(
            vec![CustomerEvent::EmailUpdated {
                new_email: "email B".to_string(),
            }],
            context,
            Default::default(),
        )
        .await
        .unwrap();
    assert_eq!(3, event_store.load_events(id.as_str()).await.unwrap().len());
}

#[tokio::test]
async fn upcasted_event() {
    let pool = create_new_db_and_pool().await;
    let upcaster = SemanticVersionEventUpcaster::new(
        "NameAdded",
        "1.0.1",
        Box::new(|mut event| match event.get_mut("NameAdded").unwrap() {
            Value::Object(object) => {
                object.insert("name".to_string(), Value::String("UNKNOWN".to_string()));
                event
            }
            _ => panic!("not the expected object"),
        }),
    );
    let event_store = new_test_event_store(pool)
        .await
        .with_upcasters(vec![Box::new(upcaster)]);

    let id = "previous_event_in_need_of_upcast".to_string();
    let result = event_store.load_aggregate(id.as_str()).await.unwrap();
    assert_eq!(1, result.current_sequence);
    assert_eq!(None, result.current_snapshot);
}
