/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        'poe-dark-gem': 'rgba(25, 5, 5, 0.6)',
        'poe-tarnished-bronze': '#8B7355',
        'poe-gold': '#FFD700',
        'poe-desaturated-gold': '#B5A642',
        'text-pure-white': '#FFFFFF',
      },
      boxShadow: {
        'text-glow': '0 0 8px rgba(255, 255, 255, 0.4)',
      },
    },
  },
  plugins: [],
}
