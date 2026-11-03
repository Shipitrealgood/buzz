"""Wrap a loopback staging relay in TLS for HTTPS and WSS integration checks."""
import argparse
import asyncio
import ssl

async def serve(args):
    tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    tls.minimum_version = ssl.TLSVersion.TLSv1_2
    tls.load_cert_chain(args.cert, args.key)

    async def copy(reader, writer):
        try:
            while chunk := await reader.read(65536):
                writer.write(chunk)
                await writer.drain()
        finally:
            writer.close()

    async def accept(inbound, client):
        try:
            outbound, relay = await asyncio.open_connection("127.0.0.1", args.relay_port)
            await asyncio.gather(copy(inbound, relay), copy(outbound, client), return_exceptions=True)
        except ConnectionRefusedError:
            # The relay may still be migrating its disposable database.
            pass
        finally:
            client.close()

    server = await asyncio.start_server(accept, "127.0.0.1", args.https_port, ssl=tls)
    async with server:
        await server.serve_forever()

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cert", required=True)
    parser.add_argument("--key", required=True)
    parser.add_argument("--https-port", type=int, required=True)
    parser.add_argument("--relay-port", type=int, required=True)
    asyncio.run(serve(parser.parse_args()))
