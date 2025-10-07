pub mod pool;
pub use pool::*;

#[cfg(test)]
mod testobj {
    use std::time::Duration;

    use tokio::time::sleep;

    use super::*;
    #[tokio::test]
    async fn test_fulfill() {
        tokio::spawn(async {
            println!("making delay");
            sleep(Duration::from_secs(1)).await;
            println!("Fulfilling value");
            fulfill("value".to_string()).await;
        });
        println!("Got value here {:?}", acquire::<String>().await);
    }

    #[tokio::test]
    async fn test_provide() {
        provide(
            "value".to_string(),
            Some(|value| async move {
                println!("Shutting down value {}", value);
            }),
        )
        .await;
        println!("Got value here {:?}", require::<String>().await);
        println!("Dropping value");
        drop_value::<String>().await;
    }
}
