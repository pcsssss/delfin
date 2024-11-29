use reqwest::{Client, ClientBuilder};

use super::api_client::APP_USER_AGENT;

pub(crate) fn get_unauthed_client_builder() -> ClientBuilder {
    #[cfg_attr(not(debug_assertions), allow(unused_mut))]
    let mut client = Client::builder().user_agent(APP_USER_AGENT);

    #[cfg(debug_assertions)]
    {
        use super::util::mitmproxy::mitmproxy_cert;
        let mitmproxy_cert = mitmproxy_cert();
        if let Some(mitmproxy_cert) = mitmproxy_cert {
            client = client.add_root_certificate(mitmproxy_cert);
        }
    }

    client
}

pub fn get_unauthed_client() -> Client {
    get_unauthed_client_builder().build().unwrap()
}
