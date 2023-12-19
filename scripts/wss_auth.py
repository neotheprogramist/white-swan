import hmac
import json
import logging
import time
import aiohttp
import asyncio

logging.basicConfig(filename='logfile_wrapper.log', level=logging.DEBUG,
                    format='%(asctime)s %(levelname)s %(message)s')
topic = "order"

async def on_message(message):
    data = json.loads(message)
    print(data)

async def on_error(error):
    print('we got error')
    print(error)

async def on_close():
    print("### about to close please don't close ###")

async def send_auth(ws):
    key = 'Ds5lWj2Rm1WyaB9JSw'
    secret = 'PKxlZy9cfeHMExthn3MnYdL73MB1Oi3O3DlS'
    expires = 1703024689167
    print(f"expires: {expires}")
    _val = f'GET/realtime{expires}'
    print(_val)
    signature = str(hmac.new(
        bytes(secret, 'utf-8'),
        bytes(_val, 'utf-8'), digestmod='sha256'
    ).hexdigest())
    print(f"signature: {signature}")
    await ws.send_str(json.dumps({"op": "auth", "args": [key, expires, signature]}))

async def on_pong(*data):
    print('pong received')

async def on_ping(*data):
    print('ping received')

async def on_open(ws):
    print('opened')
    await send_auth(ws)
    print('send subscription ' + topic)
    await ws.send_str(json.dumps({"op": "subscribe", "args": [topic]}))

async def connWS():
    async with aiohttp.ClientSession() as session:
        async with session.ws_connect("wss://stream.bybit.com/v5/private") as ws:
            await on_open(ws)
            async for msg in ws:
                if msg.type == aiohttp.WSMsgType.TEXT:
                    await on_message(msg.data)
                elif msg.type == aiohttp.WSMsgType.ERROR:
                    await on_error(ws.exception())

if __name__ == "__main__":
    loop = asyncio.get_event_loop()
    loop.run_until_complete(connWS())
