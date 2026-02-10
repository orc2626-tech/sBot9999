import { useState, useEffect } from 'react';
import { Search, Star, TrendingUp, TrendingDown } from 'lucide-react';
import { binanceService } from '../services/binance';
import { useStore } from '../store/useStore';
import { Ticker } from '../types';
import './MarketWatch.css';

const POPULAR_PAIRS = [
  'BTCUSDT',
  'ETHUSDT',
  'BNBUSDT',
  'SOLUSDT',
  'ADAUSDT',
  'XRPUSDT',
  'DOGEUSDT',
  'DOTUSDT',
  'MATICUSDT',
  'LINKUSDT',
  'UNIUSDT',
  'LTCUSDT',
];

export default function MarketWatch() {
  const { selectedSymbol, setSelectedSymbol } = useStore();
  const [tickers, setTickers] = useState<Ticker[]>([]);
  const [filteredTickers, setFilteredTickers] = useState<Ticker[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [loading, setLoading] = useState(true);
  const [favorites, setFavorites] = useState<Set<string>>(new Set());

  useEffect(() => {
    binanceService.getAllTickers().then((allTickers) => {
      const popularTickers = allTickers.filter((t) =>
        POPULAR_PAIRS.includes(t.symbol)
      );
      setTickers(popularTickers);
      setFilteredTickers(popularTickers);
      setLoading(false);
    });

    const interval = setInterval(() => {
      binanceService.getAllTickers().then((allTickers) => {
        const popularTickers = allTickers.filter((t) =>
          POPULAR_PAIRS.includes(t.symbol)
        );
        setTickers(popularTickers);
        if (!searchQuery) {
          setFilteredTickers(popularTickers);
        }
      });
    }, 3000);

    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    if (searchQuery) {
      const filtered = tickers.filter((ticker) =>
        ticker.symbol.toLowerCase().includes(searchQuery.toLowerCase())
      );
      setFilteredTickers(filtered);
    } else {
      setFilteredTickers(tickers);
    }
  }, [searchQuery, tickers]);

  const toggleFavorite = (symbol: string) => {
    const newFavorites = new Set(favorites);
    if (newFavorites.has(symbol)) {
      newFavorites.delete(symbol);
    } else {
      newFavorites.add(symbol);
    }
    setFavorites(newFavorites);
  };

  const formatSymbol = (symbol: string) => {
    return symbol.replace('USDT', '/USDT');
  };

  return (
    <div className="market-watch">
      <div className="market-watch-header">
        <h3>Markets</h3>
        <div className="search-box">
          <Search size={16} />
          <input
            type="text"
            placeholder="Search markets..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>
      </div>

      {loading ? (
        <div className="loading-container">
          <div className="loading-spinner" />
        </div>
      ) : (
        <div className="markets-list">
          <div className="markets-header">
            <span>Pair</span>
            <span>Price</span>
            <span>24h %</span>
          </div>

          {filteredTickers.map((ticker) => {
            const priceChange = parseFloat(ticker.priceChangePercent);
            const isPositive = priceChange >= 0;

            return (
              <div
                key={ticker.symbol}
                className={`market-row ${selectedSymbol === ticker.symbol ? 'active' : ''}`}
                onClick={() => setSelectedSymbol(ticker.symbol)}
              >
                <div className="market-info">
                  <button
                    className={`favorite-btn ${favorites.has(ticker.symbol) ? 'active' : ''}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      toggleFavorite(ticker.symbol);
                    }}
                  >
                    <Star size={14} />
                  </button>
                  <span className="symbol">{formatSymbol(ticker.symbol)}</span>
                </div>

                <span className="price">${parseFloat(ticker.price).toFixed(2)}</span>

                <div className={`change ${isPositive ? 'positive' : 'negative'}`}>
                  {isPositive ? <TrendingUp size={14} /> : <TrendingDown size={14} />}
                  <span>{isPositive ? '+' : ''}{priceChange.toFixed(2)}%</span>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
