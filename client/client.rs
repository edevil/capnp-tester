// Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use crate::hello_world_capnp::{first_level, hello_world};
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use log::info;

use futures::AsyncReadExt;
use tokio::{
    net::UnixStream,
    time::{sleep, Duration},
};

struct CallBacker {}

impl hello_world::hello_request::callback::Server for CallBacker {
    fn do_callback(
        &mut self,
        params: hello_world::hello_request::callback::DoCallbackParams,
        mut results: hello_world::hello_request::callback::DoCallbackResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        let text_param = pry!(pry!(params.get()).get_text_param());

        results
            .get()
            .init_response()
            .set_callback_message(&format!("CALLBACK {}", text_param));
        capnp::capability::Promise::ok(())
    }
}

pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 4 {
        println!("usage: {} client SOCKET_PATH MESSAGE", args[0]);
        return Ok(());
    }

    let addr = args[2].to_string();

    let msg = args[3].to_string();

    tokio::task::LocalSet::new()
        .run_until(async move {
            let stream = UnixStream::connect(addr).await?;
            let (reader, writer) =
                tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
            let rpc_network = Box::new(twoparty::VatNetwork::new(
                reader,
                writer,
                rpc_twoparty_capnp::Side::Client,
                Default::default(),
            ));
            let mut rpc_system = RpcSystem::new(rpc_network, None);
            let first_level: first_level::Client =
                rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

            tokio::task::spawn_local(rpc_system);

            loop {
                let first_request = first_level
                    .get_first_level_request()
                    .send()
                    .pipeline
                    .get_first();

                let second_request = first_request
                    .get_second_level_request()
                    .send()
                    .pipeline
                    .get_second();

                let hello_world = second_request
                    .get_final_request()
                    .send()
                    .pipeline
                    .get_hello_world();

                let mut request = hello_world.say_hello_request();
                let mut params = request.get().init_request();
                params.set_name(&msg);
                params.set_callback_cap(capnp_rpc::new_client(CallBacker {}));

                let reply = match tokio::time::timeout(
                    Duration::from_secs(10 as u64),
                    request.send().promise,
                )
                .await
                {
                    Ok(response) => response?,
                    Err(err) => {
                        info!("error receiving response: {}", err);
                        continue;
                    }
                };

                info!(
                    "received: {}",
                    reply
                        .get()
                        .unwrap()
                        .get_reply()
                        .unwrap()
                        .get_message()
                        .unwrap()
                );

                sleep(Duration::from_millis(1000)).await;
            }
        })
        .await
}
