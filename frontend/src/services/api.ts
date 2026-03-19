import axios from 'axios';

const api = axios.create({
  baseURL: '/api/v1',
  headers: {
    'Content-Type': 'application/json',
  },
});

// Attach JWT token from localStorage to every request
api.interceptors.request.use((config) => {
  const token = localStorage.getItem('auth_token');
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

// Track whether we're already redirecting to avoid multiple redirects
// from concurrent 401 responses (e.g., notification polling + page load)
let isRedirecting = false;

// Handle 401 responses by clearing token and redirecting to login
api.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401 && !isRedirecting) {
      isRedirecting = true;
      localStorage.removeItem('auth_token');
      // Small delay to let any pending requests settle
      setTimeout(() => {
        window.location.href = '/login';
      }, 100);
    }
    return Promise.reject(error);
  }
);

export default api;
