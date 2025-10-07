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
            provide(
                "value".to_string(),
                Some(|value| async move {
                    println!("Shutting down value {}", value);
                }),
            )
            .await;
            sleep(Duration::from_secs(1)).await;
            println!("Fulfilling value again");
            fulfill("value2".to_string()).await;
        });
        println!("Got value here {:?}", acquire::<String>().await);
        // After acquiring, drop this value
        println!("Dropping value");
        drop_value::<String>().await;
        println!("Acquired value again {:?}", acquire::<String>().await);

        shutdown().await;
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
        provide(
            1usize,
            Some(|value| async move {
                println!("Shutting down value {}", value);
            }),
        )
        .await;
        println!("Got `String` value here {:?}", require::<String>().await);
        println!("Got `usize` value here {:?}", require::<usize>().await);
        println!("Dropping `String` value");
        drop_value::<String>().await;
    }
}
