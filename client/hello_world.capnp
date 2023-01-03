@0x9663f4dd604afa35;

interface FirstLevel {
    getSecondLevel @0 () -> (second :SecondLevel);
    getFirstLevel @1 () -> (first :FirstLevel);
}

interface SecondLevel {
    getFinal @0 () -> (helloWorld :HelloWorld);
}

interface HelloWorld {
    struct HelloRequest {

        interface Callback {

            struct CallbackResponse {
                callbackMessage @0 :Text;
            }

            doCallback @0 (textParam :Text) -> (response :CallbackResponse);
        }

        name @0 :Text;
        callbackCap @1 :Callback;
    }

    struct HelloReply {
        message @0 :Text;
    }

    sayHello @0 (request: HelloRequest) -> (reply: HelloReply);
}