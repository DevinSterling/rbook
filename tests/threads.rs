#[cfg(feature = "multi-thread")]
mod multi_thread {
    use rbook::Ebook;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn multi_thread_test() {
        let epub = Arc::new(rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap());
        let mut handles = Vec::new();

        for i in 1..=5 {
            let epub = Arc::clone(&epub);
            handles.push(thread::spawn(move || {
                epub.metadata().elements().into_iter().for_each(|metadata| {
                    println!("Thread {i}: {metadata:?}");
                })
            }));
        }

        handles
            .into_iter()
            .for_each(|handle| handle.join().unwrap());
    }
}
