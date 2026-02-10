import { useEffect, useState } from 'react';
import { binanceService } from '../services/binance';
import { useStore } from '../store/useStore';
import { OrderBook as OrderBookType } from '../types';
import './OrderBook.css';

export default function OrderBook() {
  const { selectedSymbol } = useStore();
  const [orderBook, setOrderBook] = useState<OrderBookType | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);

    binanceService.getOrderBook(selectedSymbol, 20).then((data) => {
      if (data) {
        setOrderBook(data);
        setLoading(false);
      }
    });

    const interval = setInterval(() => {
      binanceService.getOrderBook(selectedSymbol, 20).then((data) => {
        if (data) {
          setOrderBook(data);
        }
      });
    }, 2000);

    return () => clearInterval(interval);
  }, [selectedSymbol]);

  const formatPrice = (price: string) => parseFloat(price).toFixed(2);
  const formatAmount = (amount: string) => parseFloat(amount).toFixed(6);

  if (loading || !orderBook) {
    return (
      <div className="order-book">
        <div className="order-book-header">
          <h3>Order Book</h3>
        </div>
        <div className="loading-container">
          <div className="loading-spinner" />
        </div>
      </div>
    );
  }

  const maxBidVolume = Math.max(...orderBook.bids.map(b => parseFloat(b[1])));
  const maxAskVolume = Math.max(...orderBook.asks.map(a => parseFloat(a[1])));

  return (
    <div className="order-book">
      <div className="order-book-header">
        <h3>Order Book</h3>
        <div className="order-book-legend">
          <span className="legend-item">
            <span className="legend-color bid" />
            Bid
          </span>
          <span className="legend-item">
            <span className="legend-color ask" />
            Ask
          </span>
        </div>
      </div>

      <div className="order-book-labels">
        <span>Price (USDT)</span>
        <span>Amount</span>
        <span>Total</span>
      </div>

      <div className="order-book-content">
        <div className="asks">
          {orderBook.asks.slice(0, 15).reverse().map((ask, index) => {
            const price = parseFloat(ask[0]);
            const amount = parseFloat(ask[1]);
            const total = price * amount;
            const percentage = (amount / maxAskVolume) * 100;

            return (
              <div key={index} className="order-row ask">
                <div className="volume-bar ask" style={{ width: `${percentage}%` }} />
                <span className="price">{formatPrice(ask[0])}</span>
                <span className="amount">{formatAmount(ask[1])}</span>
                <span className="total">{total.toFixed(2)}</span>
              </div>
            );
          })}
        </div>

        <div className="spread">
          {orderBook.asks.length > 0 && orderBook.bids.length > 0 && (
            <>
              <span className="spread-value">
                {(parseFloat(orderBook.asks[0][0]) - parseFloat(orderBook.bids[0][0])).toFixed(2)}
              </span>
              <span className="spread-label">Spread</span>
            </>
          )}
        </div>

        <div className="bids">
          {orderBook.bids.slice(0, 15).map((bid, index) => {
            const price = parseFloat(bid[0]);
            const amount = parseFloat(bid[1]);
            const total = price * amount;
            const percentage = (amount / maxBidVolume) * 100;

            return (
              <div key={index} className="order-row bid">
                <div className="volume-bar bid" style={{ width: `${percentage}%` }} />
                <span className="price">{formatPrice(bid[0])}</span>
                <span className="amount">{formatAmount(bid[1])}</span>
                <span className="total">{total.toFixed(2)}</span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
