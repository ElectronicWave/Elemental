pub mod context;
pub mod facade;
pub mod instant;
pub mod pool;
pub use facade::*;

#[cfg(test)]
mod testobj {
    use std::time::Duration;
    use tokio::{join, time::sleep};

    use super::*;
    #[tokio::test]
    async fn test_fulfill() {
        tokio::spawn(async {
            println!("making delay");
            sleep(Duration::from_secs(1)).await;
            println!("Fulfilling value");
            provide_with_shutdown("value".to_string(), |value| async move {
                println!("Shutting down value {}", value);
            })
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
        provide_with_shutdown("value".to_string(), |value| async move {
            println!("Shutting down value {}", value);
        })
        .await;
        provide_with_shutdown(1usize, |value| async move {
            println!("Shutting down value {}", value);
        })
        .await;
        println!("Got `String` value here {:?}", require::<String>().await);
        println!("Got `usize` value here {:?}", require::<usize>().await);
        println!("Dropping `String` value");
        drop_value::<String>().await;
    }

    #[tokio::test]
    async fn test_context() {
        provide("Cheese".to_string()).await;
        let context = ObjectContext::new();
        let supplyer = context.clone().run(async {
            println!("Making delay in context");
            sleep(Duration::from_secs(3)).await;
            provide_context_with_shutdown("Hamburger".to_string(), |value| async move {
                println!("Shutting down value {}", value);
            })
            .await;
        });
        let comsumer = context.clone().run(async {
            let order = acquire::<String>().await;
            println!("Got value in context {:?}", order);
        });
        join!(supplyer, comsumer);
        println!(
            "After context shutdown, value is {:?}",
            require::<String>().await
        );
        context.shutdown().await;
    }
}
