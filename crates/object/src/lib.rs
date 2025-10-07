pub mod pool;
pub use pool::*;

#[cfg(test)]
mod testobj {
    use std::time::Duration;

    use tokio::time::sleep;

    use super::*;
    #[tokio::test]
    async fn test() {
        tokio::spawn(async {
            println!("making delay");
            sleep(Duration::from_secs(1)).await;
            println!("Fulfilling value");
            fulfill("value".to_string()).await;
        });
        println!("Got value here {:?}", acquire::<String>().await);
    }
}
#[cfg(test)]
mod testmap {
    use scc::HashMap;
    use std::sync::LazyLock;
    static GLOBAL_STATIC_MAP: LazyLock<HashMap<String, String>> = LazyLock::new(|| HashMap::new());
    const GLOBAL_CONST_MAP: LazyLock<HashMap<String, String>> = LazyLock::new(|| HashMap::new());

    #[test]
    fn test_static() {
        GLOBAL_STATIC_MAP.upsert_sync("key".to_string(), "value".to_string());
        let v = GLOBAL_STATIC_MAP.read_sync(&"key".to_string(), |_, v| v.clone());
        assert_eq!(v, Some("value".to_string()));
    }

    #[test]
    fn test_const() {
        GLOBAL_CONST_MAP.upsert_sync("key".to_string(), "value".to_string());
        let v = GLOBAL_CONST_MAP.read_sync(&"key".to_string(), |_, v| v.clone());
        assert_eq!(v, Some("value".to_string()));
    }
}
