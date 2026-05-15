import { fetchStats } from './github-stats.js';
import { initScrollEngine } from './scroll-engine.js';
import { initStoryChapters, initSystemTime, initSimulator } from './story-chapters.js';
import { initTypewriter } from './typewriter.js';

document.addEventListener('DOMContentLoaded', () => {
  const loader = document.getElementById('loader');
  const loaderBar = document.getElementById('loader-progress');
  const loaderBarBg = document.getElementById('loader-bar-bg');
  const startBtn = document.getElementById('start-btn');
  
  let progress = 0;
  const interval = setInterval(() => {
    progress += Math.random() * 25;
    if (progress >= 100) {
      progress = 100;
      clearInterval(interval);
      
      // Loading complete: Show Start Button
      setTimeout(() => {
        loaderBarBg.style.display = 'none';
        startBtn.style.opacity = '1';
        startBtn.style.pointerEvents = 'auto';
        startBtn.style.transform = 'translateY(0)';
      }, 400);
    }
    loaderBar.style.width = `${progress}%`;
  }, 80);

  startBtn.addEventListener('click', () => {
    // This click event satisfies the browser's User Gesture requirement for Audio
    loader.style.opacity = '0';
    setTimeout(() => {
      loader.style.display = 'none';
      
      // Initialize everything only AFTER the user has engaged
      fetchStats();
      initSystemTime();
      initScrollEngine();
      initStoryChapters();
      initSimulator();
      initTypewriter();
    }, 600);
  });

  // Time-of-Day Theme (Dark mode after 7PM, before 6AM)
  const hour = new Date().getHours();
  if (hour >= 19 || hour < 6) {
    document.documentElement.classList.add('theme-dark');
  }

  // OS Detection Fallback
  const isMacOrLinux = /Mac|Linux/i.test(navigator.userAgent || navigator.platform);
  if (isMacOrLinux) {
    document.querySelectorAll('[data-action="download"]').forEach(btn => {
      const textSpan = btn.querySelector('[data-text="download"]');
      if (textSpan) textSpan.textContent = 'Windows Only — View GitHub';
      btn.href = 'https://github.com/0xarchit/pauseCat';
      btn.style.background = 'var(--bg-ink-wash)';
      btn.style.color = 'white';
    });
  }

  // Terminal Copy Functionality
  const copyBtn = document.getElementById('terminal-copy');
  if (copyBtn) {
    copyBtn.addEventListener('click', () => {
      const command = 'git clone https://github.com/0xarchit/pauseCat';
      navigator.clipboard.writeText(command).then(() => {
        const span = copyBtn.querySelector('span');
        const oldText = span.textContent;
        span.textContent = 'Copied!';
        copyBtn.classList.add('copied');
        setTimeout(() => {
          span.textContent = oldText;
          copyBtn.classList.remove('copied');
        }, 2000);
      });
    });
  }
});
