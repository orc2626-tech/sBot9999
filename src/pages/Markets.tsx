import { useState, useEffect } from 'react';
import { Search, TrendingUp, TrendingDown, Star, ArrowUpDown } from 'lucide-react';
import { binanceService } from '../services/binance';
import { Ticker } from '../types';
import { useStore } from '../store/useStore';
import { useNavigate } from 'react-router-dom';
import './Markets.css';

export default function Markets() {
  const [tickers, setTickers] = useState<Ticker[]>([]);
  const [filteredTickers, setFilteredTickers] = useState<Ticker[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [loading, setLoading] = useState(true);
  const [sortBy, setSortBy] = useState<'volume' | 'price' | 'change'>('volume');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc');
  const [favorites, setFavorites] = useState<Set<string>>(new Set());
  const { setSelectedSymbol } = useStore();
  const navigate = useNavigate();

  useEffect(() => {
    binanceService.getAllTickers().then((allTickers) => {
      const usdtTickers = allTickers.filter((t) => t.symbol.includes('USDT'));
      setTickers(usdtTickers);
      setFilteredTickers(usdtTickers);
      setLoading(false);
    });

    const interval = setInterval(() => {
      binanceService.getAllTickers().then((allTickers) => {
        const usdtTickers = allTickers.filter((t) => t.symbol.includes('USDT'));
        setTickers(usdtTickers);
        if (!searchQuery) {
          setFilteredTickers(usdtTickers);
        }
      });
    }, 5000);

    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    let filtered = [...tickers];

    if (searchQuery) {
      filtered = filtered.filter((ticker) =>
        ticker.symbol.toLowerCase().includes(searchQuery.toLowerCase())
      );
    }

    filtered.sort((a, b) => {
      let aVal, bVal;

      switch (sortBy) {
        case 'volume':
          aVal = parseFloat(a.volume);
          bVal = parseFloat(b.volume);
          break;
        case 'price':
          aVal = parseFloat(a.price);
          bVal = parseFloat(b.price);
          break;
        case 'change':
          aVal = parseFloat(a.priceChangePercent);
          bVal = parseFloat(b.priceChangePercent);
          break;
        default:
          aVal = 0;
          bVal = 0;
      }

      return sortOrder === 'asc' ? aVal - bVal : bVal - aVal;
    });

    setFilteredTickers(filtered);
  }, [searchQuery, tickers, sortBy, sortOrder]);

  const toggleSort = (field: 'volume' | 'price' | 'change') => {
    if (sortBy === field) {
      setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
    } else {
      setSortBy(field);
      setSortOrder('desc');
    }
  };

  const toggleFavorite = (symbol: string) => {
    const newFavorites = new Set(favorites);
    if (newFavorites.has(symbol)) {
      newFavorites.delete(symbol);
    } else {
      newFavorites.add(symbol);
    }
    setFavorites(newFavorites);
  };

  const handleTradeClick = (symbol: string) => {
    setSelectedSymbol(symbol);
    navigate('/trading');
  };

  const topGainers = [...tickers]
    .sort((a, b) => parseFloat(b.priceChangePercent) - parseFloat(a.priceChangePercent))
    .slice(0, 3);

  const topLosers = [...tickers]
    .sort((a, b) => parseFloat(a.priceChangePercent) - parseFloat(b.priceChangePercent))
    .slice(0, 3);

  return (
    <div className="markets-page">
      <div className="markets-header">
        <div>
          <h1>Markets</h1>
          <p>Explore and trade cryptocurrency markets</p>
        </div>

        <div className="search-container">
          <Search size={20} />
          <input
            type="text"
            placeholder="Search markets..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>
      </div>

      <div className="market-highlights">
        <div className="highlight-card">
          <h3>
            <TrendingUp className="icon-green" />
            Top Gainers
          </h3>
          <div className="highlight-list">
            {topGainers.map((ticker) => (
              <div key={ticker.symbol} className="highlight-item">
                <span className="symbol">{ticker.symbol.replace('USDT', '')}</span>
                <span className="change positive">
                  +{parseFloat(ticker.priceChangePercent).toFixed(2)}%
                </span>
              </div>
            ))}
          </div>
        </div>

        <div className="highlight-card">
          <h3>
            <TrendingDown className="icon-red" />
            Top Losers
          </h3>
          <div className="highlight-list">
            {topLosers.map((ticker) => (
              <div key={ticker.symbol} className="highlight-item">
                <span className="symbol">{ticker.symbol.replace('USDT', '')}</span>
                <span className="change negative">
                  {parseFloat(ticker.priceChangePercent).toFixed(2)}%
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="markets-table-container">
        <div className="markets-table">
          <div className="table-header">
            <span className="favorite-col"></span>
            <span>Pair</span>
            <span className="sortable" onClick={() => toggleSort('price')}>
              Price
              {sortBy === 'price' && <ArrowUpDown size={14} />}
            </span>
            <span className="sortable" onClick={() => toggleSort('change')}>
              24h Change
              {sortBy === 'change' && <ArrowUpDown size={14} />}
            </span>
            <span>24h High</span>
            <span>24h Low</span>
            <span className="sortable" onClick={() => toggleSort('volume')}>
              24h Volume
              {sortBy === 'volume' && <ArrowUpDown size={14} />}
            </span>
            <span className="action-col"></span>
          </div>

          <div className="table-body">
            {loading ? (
              <div className="loading-state">
                <div className="loading-spinner" />
                <p>Loading markets...</p>
              </div>
            ) : (
              filteredTickers.map((ticker) => {
                const priceChange = parseFloat(ticker.priceChangePercent);
                const isPositive = priceChange >= 0;

                return (
                  <div
                    key={ticker.symbol}
                    className="table-row"
                    onClick={() => handleTradeClick(ticker.symbol)}
                  >
                    <button
                      className={`favorite-btn ${favorites.has(ticker.symbol) ? 'active' : ''}`}
                      onClick={(e) => {
                        e.stopPropagation();
                        toggleFavorite(ticker.symbol);
                      }}
                    >
                      <Star size={16} />
                    </button>

                    <span className="pair-cell">
                      <span className="symbol-main">{ticker.symbol.replace('USDT', '')}</span>
                      <span className="symbol-quote">/USDT</span>
                    </span>

                    <span className="price-cell">
                      ${parseFloat(ticker.price).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                    </span>

                    <span className={`change-cell ${isPositive ? 'positive' : 'negative'}`}>
                      {isPositive ? <TrendingUp size={14} /> : <TrendingDown size={14} />}
                      {isPositive ? '+' : ''}{priceChange.toFixed(2)}%
                    </span>

                    <span className="high-cell">
                      ${parseFloat(ticker.high).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                    </span>

                    <span className="low-cell">
                      ${parseFloat(ticker.low).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                    </span>

                    <span className="volume-cell">
                      ${parseFloat(ticker.volume).toLocaleString('en-US', { maximumFractionDigits: 0 })}
                    </span>

                    <button
                      className="trade-btn"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleTradeClick(ticker.symbol);
                      }}
                    >
                      Trade
                    </button>
                  </div>
                );
              })
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
