import { create } from 'zustand';
import { User, Position, Balance, MarketStats, Trade } from '../types';

interface AppStore {
  user: User | null;
  setUser: (user: User | null) => void;

  selectedSymbol: string;
  setSelectedSymbol: (symbol: string) => void;

  positions: Position[];
  setPositions: (positions: Position[]) => void;

  balances: Balance[];
  setBalances: (balances: Balance[]) => void;

  marketStats: MarketStats;
  setMarketStats: (stats: MarketStats) => void;

  trades: Trade[];
  setTrades: (trades: Trade[]) => void;

  chartInterval: string;
  setChartInterval: (interval: string) => void;

  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
}

export const useStore = create<AppStore>((set) => ({
  user: null,
  setUser: (user) => set({ user }),

  selectedSymbol: 'BTCUSDT',
  setSelectedSymbol: (symbol) => set({ selectedSymbol: symbol }),

  positions: [],
  setPositions: (positions) => set({ positions }),

  balances: [],
  setBalances: (balances) => set({ balances }),

  marketStats: {
    totalBalance: 0,
    totalPnL: 0,
    totalPnLPercentage: 0,
    dayChange: 0,
    dayChangePercentage: 0,
  },
  setMarketStats: (marketStats) => set({ marketStats }),

  trades: [],
  setTrades: (trades) => set({ trades }),

  chartInterval: '1h',
  setChartInterval: (interval) => set({ chartInterval: interval }),

  sidebarCollapsed: false,
  toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
}));
