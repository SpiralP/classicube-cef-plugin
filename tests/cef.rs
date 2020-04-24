// #[test]
// fn test_cef() {
//     use crate::cef::AsyncManager;

//     crate::logger::initialize(true, false);

//     unsafe {
//         extern "C" fn ag(_: *mut classicube_sys::ScheduledTask) {}
//         classicube_sys::Server.Tick = Some(ag);
//     }

//     let mut am = AsyncManager::new();
//     am.initialize();

//     let is_shutdown = std::rc::Rc::new(std::cell::Cell::new(false));
//     let is_initialized = std::rc::Rc::new(std::cell::Cell::new(false));

//     {
//         let is_shutdown = is_shutdown.clone();
//         let is_initialized = is_initialized.clone();
//         AsyncManager::spawn_local_on_main_thread(async move {
//             let app = {
//                 debug!("create");
//                 let (app, client) = create_app().await;
//                 is_initialized.set(true);

//                 // {
//                 //     let is_shutdown = is_shutdown.clone();
//                 //     AsyncManager::spawn_local_on_main_thread(async move {
//                 //         while !is_shutdown.get() {
//                 //             debug!(".");
//                 //             RustRefApp::step().unwrap();
//                 //             // tokio::task::yield_now().await;
//                 //             async_std::task::yield_now().await;
//                 //         }
//                 //     });
//                 // }

//                 debug!("create_browser 1");
//                 let browser_1 = create_browser(&client, "https://icanhazip.com/".to_string());
//                 debug!("create_browser 2");
//                 let browser_2 = create_browser(&client, "https://icanhazip.com/".to_string());

//                 let (browser_1, browser_2) = futures::future::join(browser_1, browser_2).await;

//                 let never = futures::future::pending::<()>();
//                 let dur = std::time::Duration::from_secs(2);
//                 assert!(async_std::future::timeout(dur, never).await.is_err());

//                 debug!("browsers close");
//                 futures::future::join_all(
//                     [browser_1, browser_2]
//                         .iter()
//                         .map(|browser| close_browser(browser)),
//                 )
//                 .await;
//                 // close_browser(&browser_1).await;
//                 // close_browser(&browser_2).await;

//                 app
//             };

//             debug!("shutdown");
//             app.shutdown().unwrap();
//             is_shutdown.set(true);
//         });
//     }

//     while !is_shutdown.get() {
//         let before = std::time::Instant::now();
//         AsyncManager::step();
//         if is_initialized.get() && !is_shutdown.get() {
//             RustRefApp::step().unwrap();
//         }
//         debug!("step {:?}", std::time::Instant::now() - before);
//         std::thread::sleep(std::time::Duration::from_millis(100));
//     }
// }
