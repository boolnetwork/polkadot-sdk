const express = require("express");
const axios = require("axios");
const { HttpsProxyAgent } = require("https-proxy-agent");

const app = express();

const proxyAgent = new HttpsProxyAgent("http://127.0.0.1:7890");

app.get("/binance", async (req, res) => {
	console.log("binance request");
	try {
		const response = await axios.get("https://fapi.binance.com/fapi/v1/ticker/price", {
			httpsAgent: proxyAgent,
			headers: {
				"User-Agent": "Mozilla/5.0",
				Accept: "application/json",
			},
		});
		res.json(response.data);
	} catch (err) {
		console.error("binance Error during fetch:", err.message);
		res.status(500).send("Proxy failed");
	}
});

app.get("/coinbase", async (req, res) => {
	console.log("coinbase request");
	const symbol = req.query.symbol || "BTC-USDT";
	const url = `https://api.exchange.coinbase.com/products/${symbol}/ticker`;

	try {
		const response = await axios.get(url, {
			httpsAgent: proxyAgent,
			headers: {
				"User-Agent": "Mozilla/5.0",
				Accept: "application/json",
			},
		});
		res.json(response.data);
	} catch (err) {
		console.error("coinbase Error during fetch:", err.message);
		res.status(500).send("Proxy failed");
	}
});

app.get("/bybit", async (req, res) => {
	console.log("bybit request");
	try {
		const response = await axios.get(
			"https://api.bybit.com/v5/market/tickers?category=linear",
			{
				httpsAgent: proxyAgent,
				headers: {
					"User-Agent": "Mozilla/5.0",
					Accept: "application/json",
				},
			}
		);
		res.json(response.data);
	} catch (err) {
		console.error("bybit Error during fetch:", err.message);
		res.status(500).send("Proxy failed");
	}
});

app.listen(3000, () => console.log("Proxy server running on port 3000"));
