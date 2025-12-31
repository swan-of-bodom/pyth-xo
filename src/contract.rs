// ABI of pyth contract: https://docs.pyth.network/price-feeds/core/contract-addresses/evm

use alloy::sol;

sol!(
    #[sol(rpc)]
    IPythContract,
    r#"[
        {
            "inputs": [{"internalType": "bytes[]", "name": "updateData", "type": "bytes[]"}],
            "name": "updatePriceFeeds",
            "outputs": [],
            "stateMutability": "payable",
            "type": "function"
        },
        {
            "inputs": [{"internalType": "bytes[]", "name": "updateData", "type": "bytes[]"}],
            "name": "getUpdateFee",
            "outputs": [{"internalType": "uint256", "name": "feeAmount", "type": "uint256"}],
            "stateMutability": "view",
            "type": "function"
        },
        {
            "inputs": [{"internalType": "bytes32", "name": "id", "type": "bytes32"}],
            "name": "getPriceUnsafe",
            "outputs": [
                {"internalType": "int64", "name": "price", "type": "int64"},
                {"internalType": "uint64", "name": "conf", "type": "uint64"},
                {"internalType": "int32", "name": "expo", "type": "int32"},
                {"internalType": "uint256", "name": "publishTime", "type": "uint256"}
            ],
            "stateMutability": "view",
            "type": "function"
        }
    ]"#
);
