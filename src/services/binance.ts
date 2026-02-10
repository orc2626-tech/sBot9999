import axios from 'axios';
import { Ticker, OrderBook, Kline } from '../types';

const BINANCE_API = 'https://api.binance.com/api/v3';
const BINANCE_WS = 'wss://stream.binance.com:9443/ws';

export class BinanceService {
  private ws: WebSocket | null = null;
  private subscribers: Map<string, Set<(data: any) => void>> = new Map();

  async getTicker(symbol: string): Promise<Ticker | null> {
    try {
      const response = await axios.get(`${BINANCE_API}/ticker/24hr`, {
        params: { symbol }
      });
      return response.data;
    } catch (error) {
      console.error('Error fetching ticker:', error);
      return null;
    }
  }

  async getAllTickers(): Promise<Ticker[]> {
    try {
      const response = await axios.get(`${BINANCE_API}/ticker/24hr`);
      return response.data;
    } catch (error) {
      console.error('Error fetching tickers:', error);
      return [];
    }
  }

  async getOrderBook(symbol: string, limit: number = 20): Promise<OrderBook | null> {
    try {
      const response = await axios.get(`${BINANCE_API}/depth`, {
        params: { symbol, limit }
      });
      return response.data;
    } catch (error) {
      console.error('Error fetching order book:', error);
      return null;
    }
  }

  async getKlines(
    symbol: string,
    interval: string = '1h',
    limit: number = 100
  ): Promise<Kline[]> {
    try {
      const response = await axios.get(`${BINANCE_API}/klines`, {
        params: { symbol, interval, limit }
      });

      return response.data.map((k: any[]) => ({
        time: k[0] / 1000,
        open: parseFloat(k[1]),
        high: parseFloat(k[2]),
        low: parseFloat(k[3]),
        close: parseFloat(k[4]),
        volume: parseFloat(k[5])
      }));
    } catch (error) {
      console.error('Error fetching klines:', error);
      return [];
    }
  }

  subscribeToTicker(symbol: string, callback: (ticker: Ticker) => void) {
    const stream = `${symbol.toLowerCase()}@ticker`;

    if (!this.subscribers.has(stream)) {
      this.subscribers.set(stream, new Set());
    }

    this.subscribers.get(stream)!.add(callback);

    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      this.connectWebSocket();
    } else {
      this.subscribeToStream(stream);
    }

    return () => {
      const subs = this.subscribers.get(stream);
      if (subs) {
        subs.delete(callback);
        if (subs.size === 0) {
          this.unsubscribeFromStream(stream);
          this.subscribers.delete(stream);
        }
      }
    };
  }

  subscribeToKline(symbol: string, interval: string, callback: (kline: Kline) => void) {
    const stream = `${symbol.toLowerCase()}@kline_${interval}`;

    if (!this.subscribers.has(stream)) {
      this.subscribers.set(stream, new Set());
    }

    this.subscribers.get(stream)!.add((data: any) => {
      const k = data.k;
      callback({
        time: k.t / 1000,
        open: parseFloat(k.o),
        high: parseFloat(k.h),
        low: parseFloat(k.l),
        close: parseFloat(k.c),
        volume: parseFloat(k.v)
      });
    });

    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      this.connectWebSocket();
    } else {
      this.subscribeToStream(stream);
    }

    return () => {
      const subs = this.subscribers.get(stream);
      if (subs) {
        subs.delete(callback);
        if (subs.size === 0) {
          this.unsubscribeFromStream(stream);
          this.subscribers.delete(stream);
        }
      }
    };
  }

  private connectWebSocket() {
    if (this.ws) {
      this.ws.close();
    }

    this.ws = new WebSocket(`${BINANCE_WS}/!ticker@arr`);

    this.ws.onopen = () => {
      console.log('WebSocket connected');
      this.subscribers.forEach((_, stream) => {
        this.subscribeToStream(stream);
      });
    };

    this.ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);

        if (Array.isArray(data)) {
          data.forEach(ticker => {
            const stream = `${ticker.s.toLowerCase()}@ticker`;
            const subs = this.subscribers.get(stream);
            if (subs) {
              subs.forEach(callback => callback(ticker));
            }
          });
        } else if (data.stream) {
          const subs = this.subscribers.get(data.stream);
          if (subs) {
            subs.forEach(callback => callback(data.data));
          }
        }
      } catch (error) {
        console.error('WebSocket message error:', error);
      }
    };

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };

    this.ws.onclose = () => {
      console.log('WebSocket closed, reconnecting...');
      setTimeout(() => this.connectWebSocket(), 5000);
    };
  }

  private subscribeToStream(stream: string) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({
        method: 'SUBSCRIBE',
        params: [stream],
        id: Date.now()
      }));
    }
  }

  private unsubscribeFromStream(stream: string) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({
        method: 'UNSUBSCRIBE',
        params: [stream],
        id: Date.now()
      }));
    }
  }

  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.subscribers.clear();
  }
}

export const binanceService = new BinanceService();
