/** @type {import('tailwindcss').Config} */
export default {
  darkMode: 'class',
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        mab: {
          bg: '#0D0E11',
          sidebar: '#131418',
          surface: '#18191E',
          hover: '#202229',
          elevated: '#252732',
          active: '#2D303D',
          border: 'rgba(255, 255, 255, 0.07)',
          'border-input': 'rgba(255, 255, 255, 0.10)',
          'border-hover': 'rgba(255, 255, 255, 0.18)',

          focus: '#3B82F6',
          break: '#10B981',
          success: '#10B981',
          overtime: '#F59E0B',
          warning: '#F59E0B',
          draft: '#F59E0B',
          edited: '#8B5CF6',
          danger: '#F43F5E',

          text: {
            primary: '#F3F4F6',
            secondary: '#9CA3AF',
            muted: '#646675',
          },

          tint: {
            'focus-12': 'rgba(59, 130, 246, 0.10)',
            'focus-24': 'rgba(59, 130, 246, 0.20)',
            'break-12': 'rgba(16, 185, 129, 0.10)',
            'warning-10': 'rgba(245, 158, 11, 0.10)',
            'warning-14': 'rgba(245, 158, 11, 0.14)',
            'edited-12': 'rgba(139, 92, 246, 0.10)',
            'edited-16': 'rgba(139, 92, 246, 0.15)',
            'danger-10': 'rgba(244, 63, 94, 0.10)',
            'danger-16': 'rgba(244, 63, 94, 0.15)',
          },
        },
      },
      fontFamily: {
        sans: ['Vazirmatn', 'system-ui', '-apple-system', 'Segoe UI', 'Tahoma', 'sans-serif'],
        mono: ['JetBrains Mono', 'ui-monospace', 'SFMono-Regular', 'Consolas', 'monospace'],
      },
      borderRadius: {
        'mab-badge-sm': '4px',
        'mab-badge': '6px',
        'mab-input': '10px',
        'mab-icon': '10px',
        'mab-stat': '12px',
        'mab-card': '14px',
        'mab-sheet': '18px',
        'mab-modal': '24px',
      },
      spacing: {
        'mab-titlebar': '46px',
        'mab-header': '46px',
        'mab-rail': '64px',
        'mab-rail-extended': '256px',
      },
      boxShadow: {
        'mab-card': '0 4px 20px -2px rgba(0, 0, 0, 0.35)',
        'mab-dropdown': '0 10px 30px -4px rgba(0, 0, 0, 0.6)',
        'mab-modal': '0 20px 50px -10px rgba(0, 0, 0, 0.7)',
        'mab-glow-focus': '0 0 16px rgba(59, 130, 246, 0.3)',
      },
    },
  },
  plugins: [],
};
