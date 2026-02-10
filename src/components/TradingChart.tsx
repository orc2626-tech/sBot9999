import { useEffect, useRef, useState } from 'react';
import { createChart, IChartApi, ISeriesApi } from 'lightweight-charts';
import { binanceService } from '../services/binance';
import { useStore } from '../store/useStore';
import { Kline } from '../types';
import './TradingChart.css';

const intervals = [
  { label: '1m', value: '1m' },
  { label: '5m', value: '5m' },
  { label: '15m', value: '15m' },
  { label: '1h', value: '1h' },
  { label: '4h', value: '4h' },
  { label: '1d', value: '1d' },
];

export default function TradingChart() {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<ISeriesApi<'Candlestick'> | null>(null);
  const { selectedSymbol, chartInterval, setChartInterval } = useStore();
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!chartContainerRef.current) return;

    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { color: '#131722' },
        textColor: '#787B86',
      },
      grid: {
        vertLines: { color: '#1E222D' },
        horzLines: { color: '#1E222D' },
      },
      width: chartContainerRef.current.clientWidth,
      height: 500,
      timeScale: {
        borderColor: '#2A2E39',
        timeVisible: true,
      },
      rightPriceScale: {
        borderColor: '#2A2E39',
      },
    });

    const candlestickSeries = chart.addCandlestickSeries({
      upColor: '#26A69A',
      downColor: '#EF5350',
      borderUpColor: '#26A69A',
      borderDownColor: '#EF5350',
      wickUpColor: '#26A69A',
      wickDownColor: '#EF5350',
    });

    chartRef.current = chart;
    seriesRef.current = candlestickSeries;

    const handleResize = () => {
      if (chartContainerRef.current && chartRef.current) {
        chartRef.current.applyOptions({
          width: chartContainerRef.current.clientWidth,
        });
      }
    };

    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      chart.remove();
    };
  }, []);

  useEffect(() => {
    if (!seriesRef.current) return;

    setLoading(true);

    binanceService.getKlines(selectedSymbol, chartInterval, 500).then((klines) => {
      if (seriesRef.current && klines.length > 0) {
        seriesRef.current.setData(klines as any);
        setLoading(false);
      }
    });

    const unsubscribe = binanceService.subscribeToKline(
      selectedSymbol,
      chartInterval,
      (kline: Kline) => {
        if (seriesRef.current) {
          seriesRef.current.update(kline as any);
        }
      }
    );

    return () => {
      unsubscribe();
    };
  }, [selectedSymbol, chartInterval]);

  return (
    <div className="trading-chart">
      <div className="chart-header">
        <div className="interval-selector">
          {intervals.map((interval) => (
            <button
              key={interval.value}
              className={`interval-btn ${chartInterval === interval.value ? 'active' : ''}`}
              onClick={() => setChartInterval(interval.value)}
            >
              {interval.label}
            </button>
          ))}
        </div>
      </div>

      <div className="chart-container" ref={chartContainerRef}>
        {loading && (
          <div className="chart-loading">
            <div className="loading-spinner" />
            <span>Loading chart data...</span>
          </div>
        )}
      </div>
    </div>
  );
}
