import requests
import time
import hashlib
import hmac
import uuid

api_key='Ds5lWj2Rm1WyaB9JSw'
secret_key='PKxlZy9cfeHMExthn3MnYdL73MB1Oi3O3DlS'
httpClient=requests.Session()
recv_window=str(5000)
url="https://api.bybit.com" # Testnet endpoint

def HTTP_Request(endPoint,method,payload,Info):
    global time_stamp
    time_stamp=str(int(time.time() * 10 ** 3))
    signature=genSignature(payload)
    headers = {
        'X-BAPI-API-KEY': api_key,
        'X-BAPI-SIGN': signature,
        'X-BAPI-SIGN-TYPE': '2',
        'X-BAPI-TIMESTAMP': time_stamp,
        'X-BAPI-RECV-WINDOW': recv_window,
        'Content-Type': 'application/json'
    }
    if(method=="POST"):
        response = httpClient.request(method, url+endPoint, headers=headers, data=payload)
    else:
        response = httpClient.request(method, url+endPoint+"?"+payload, headers=headers)
    print(response.text)
    print(Info + " Elapsed Time : " + str(response.elapsed))

def genSignature(payload):
    param_str= str(time_stamp) + api_key + recv_window + payload
    print(f"param_str: {param_str}")
    hash = hmac.new(bytes(secret_key, "utf-8"), param_str.encode("utf-8"),hashlib.sha256)
    signature = hash.hexdigest()
    print(f"signature: {signature}")
    return signature

#Create Order
endpoint="/v5/order/create"
method="POST"
orderLinkId=uuid.uuid4().hex
params='{"category":"spot","symbol":"MATICUSDC","side":"Buy","orderType":"Limit","qty":"1.0","price":"0.7"}'
HTTP_Request(endpoint,method,params,"Create")
