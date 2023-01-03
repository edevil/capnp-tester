#!/usr/bin/env python3

import os.path
import capnp
import logging
import asyncio

FORMAT = '%(asctime)s TEST %(message)s'
logging.basicConfig(format=FORMAT, level=logging.DEBUG)

hello_world_capnp = capnp.load('../client/hello_world.capnp')

class Server:
    async def myreader(self):
        while self.retry:
            try:
                # Must be a wait_for so we don't block on read()
                data = await asyncio.wait_for(
                    self.reader.read(4096),
                    timeout=0.1
                )
            except asyncio.TimeoutError:
                logging.debug("myreader timeout.")
                continue
            except Exception as err:
                logging.debug("Unknown myreader err: %s", err)
                return False
            logging.debug("read bytes from reader: %d", len(data))
            await self.server.write(data)
            logging.debug("wrote bytes to RPC server")
        logging.debug("myreader done.")
        return True

    async def mywriter(self):
        while self.retry:
            try:
                # Must be a wait_for so we don't block on read()
                data = await asyncio.wait_for(
                    self.server.read(4096),
                    timeout=0.1
                )
                logging.debug("read bytes from RPC server: %d", len(data))
                self.writer.write(data.tobytes())
                logging.debug("wrote bytes to writer")
            except asyncio.TimeoutError:
                logging.debug("mywriter timeout.")
                continue
            except Exception as err:
                logging.debug("Unknown mywriter err: %s", err)
                return False
        logging.debug("mywriter done.")
        return True

    async def myserver(self, reader, writer):
        # Start TwoPartyServer using TwoWayPipe (only requires bootstrap)
        self.server = capnp.TwoPartyServer(bootstrap=HelloImpl())
        self.reader = reader
        self.writer = writer
        self.retry = True

        # Assemble reader and writer tasks, run in the background
        coroutines = [self.myreader(), self.mywriter()]
        tasks = asyncio.gather(*coroutines, return_exceptions=True)

        while True:
            self.server.poll_once()
            # Check to see if reader has been sent an eof (disconnect)
            if self.reader.at_eof():
                self.retry = False
                break
            await asyncio.sleep(0.01)

        # Make wait for reader/writer to finish (prevent possible resource leaks)
        await tasks

async def new_connection(reader, writer):
    server = Server()
    await server.myserver(reader, writer)

async def main(bind_path):

    server = await asyncio.start_unix_server(new_connection, bind_path)

    async with server:
        await server.serve_forever()

class HelloImpl(hello_world_capnp.HelloWorld.Server):
    def sayHello(self, request, _context, **kwargs):

        def set_result(cb_res):
            logging.debug(cb_res.response.callbackMessage)
            reply = hello_world_capnp.HelloWorld.HelloReply()
            reply.message = cb_res.response.callbackMessage
            _context.results.reply = reply
            return

        name = request.name
        return request.callbackCap.doCallback(name).then(set_result)

if __name__ == '__main__':
    bind_path = 'sock'
    if os.path.exists(bind_path):
        os.remove(bind_path)

    asyncio.run(main(bind_path))
