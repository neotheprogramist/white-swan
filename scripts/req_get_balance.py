from pybit.unified_trading import HTTP
session = HTTP(
    api_key="Ds5lWj2Rm1WyaB9JSw",
    api_secret="PKxlZy9cfeHMExthn3MnYdL73MB1Oi3O3DlS",
)
print(session.get_wallet_balance(
    accountType="UNIFIED",
    coin="USDC",
))
