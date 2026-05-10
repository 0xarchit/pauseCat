import { fetchStats } from './github-stats.js';
import { initScrollReveal } from './scroll-reveal.js';
import { initNav } from './nav.js';
import { initTypewriter } from './typewriter.js';
import { initInteractive } from './interactive.js';

document.addEventListener('DOMContentLoaded', () => {
  fetchStats();
  initScrollReveal();
  initNav();
  initTypewriter();
  initInteractive();
});