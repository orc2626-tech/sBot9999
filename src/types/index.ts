export interface User {
  id: string;
  email: string;
  created_at: string;
}

export interface BinanceCredentials {
  id: string;
  user_id: string;
  api_key: string;
  encrypted_secret: string;
  created_at: string;
}

export interface Trade {
  id: string;
  user_id: string;
  symbol: string;
  side: 'BUY' | 'SELL';
  type: 'MARKET' | 'LIMIT' | 'STOP_LOSS' | 'STOP_LOSS_LIMIT' | 'TAKE_PROFIT' | 'TAKE_PROFIT_LIMIT';
  quantity: number;
  price?: number;
  stop_price?: number;
  status: 'PENDING' | 'FILLED' | 'CANCELLED' | 'FAILED';
  binance_order_id?: string;
  created_at: string;
  filled_at?: string;
}

export interface Position {
  symbol: string;
  quantity: number;
  entry_price: number;
  current_price: number;
  pnl: number;
  pnl_percentage: number;
}

export interface Ticker {
  symbol: string;
  price: string;
  priceChange: string;
  priceChangePercent: string;
  volume: string;
  high: string;
  low: string;
}

export interface OrderBook {
  bids: [string, string][];
  asks: [string, string][];
}

export interface Kline {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

export interface Balance {
  asset: string;
  free: string;
  locked: string;
  total: number;
  usdValue: number;
}

export interface MarketStats {
  totalBalance: number;
  totalPnL: number;
  totalPnLPercentage: number;
  dayChange: number;
  dayChangePercentage: number;
}
