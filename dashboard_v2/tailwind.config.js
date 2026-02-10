/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        bg: { primary: '#0a0e17', secondary: '#111827', tertiary: '#1a2235', elevated: '#1e293b' },
        panel: { DEFAULT: '#111827', border: '#1e3a5f', hover: '#172234' },
        panelBorder: '#1e3a5f',
        green: '#10b981',
        greenDim: '#059669',
        yellow: '#f59e0b',
        red: '#ef4444',
        blue: '#3b82f6',
        purple: '#8b5cf6',
        cyan: '#06b6d4',
        text: { primary: '#f1f5f9', secondary: '#94a3b8', muted: '#64748b' },
      },
      fontFamily: {
        sans: ['"DM Sans"', 'system-ui', 'sans-serif'],
        mono: ['"JetBrains Mono"', '"Fira Code"', 'monospace'],
        display: ['"Lexend"', '"DM Sans"', 'sans-serif'],
      },
      boxShadow: {
        glow: '0 0 20px rgba(16,185,129,0.15)',
        'glow-red': '0 0 20px rgba(239,68,68,0.15)',
        'glow-blue': '0 0 20px rgba(59,130,246,0.15)',
        card: '0 4px 6px -1px rgba(0,0,0,0.3), 0 2px 4px -2px rgba(0,0,0,0.2)',
        'card-hover': '0 10px 15px -3px rgba(0,0,0,0.4), 0 4px 6px -4px rgba(0,0,0,0.3)',
      },
      animation: {
        'pulse-slow': 'pulse 3s ease-in-out infinite',
        'fade-in': 'fadeIn 0.3s ease-out',
        'slide-up': 'slideUp 0.4s ease-out',
        'glow-pulse': 'glowPulse 2s ease-in-out infinite',
      },
      keyframes: {
        fadeIn: { '0%': { opacity: '0' }, '100%': { opacity: '1' } },
        slideUp: { '0%': { opacity: '0', transform: 'translateY(10px)' }, '100%': { opacity: '1', transform: 'translateY(0)' } },
        glowPulse: { '0%, 100%': { boxShadow: '0 0 5px rgba(16,185,129,0.2)' }, '50%': { boxShadow: '0 0 20px rgba(16,185,129,0.4)' } },
      },
    },
  },
  plugins: [],
}
