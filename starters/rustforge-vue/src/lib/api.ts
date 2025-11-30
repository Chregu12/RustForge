import axios from 'axios'

const api = axios.create({
  baseURL: '/api',
  headers: {
    'Content-Type': 'application/json',
    'Accept': 'application/json',
  },
  withCredentials: true,
})

// Request interceptor to add auth token
api.interceptors.request.use((config) => {
  const token = localStorage.getItem('token')
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

// Response interceptor for error handling
api.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      localStorage.removeItem('token')
      window.location.href = '/login'
    }
    return Promise.reject(error)
  }
)

export interface User {
  id: number
  name: string
  email: string
  email_verified_at?: string
  created_at: string
  updated_at: string
}

export interface LoginCredentials {
  email: string
  password: string
  remember?: boolean
}

export interface RegisterData {
  name: string
  email: string
  password: string
  password_confirmation: string
}

export interface AuthResponse {
  user: User
  token: string
}

export const authApi = {
  login: async (credentials: LoginCredentials): Promise<AuthResponse> => {
    const { data } = await api.post<AuthResponse>('/auth/login', credentials)
    localStorage.setItem('token', data.token)
    return data
  },

  register: async (userData: RegisterData): Promise<AuthResponse> => {
    const { data } = await api.post<AuthResponse>('/auth/register', userData)
    localStorage.setItem('token', data.token)
    return data
  },

  logout: async (): Promise<void> => {
    await api.post('/auth/logout')
    localStorage.removeItem('token')
  },

  getUser: async (): Promise<User> => {
    const { data } = await api.get<User>('/user')
    return data
  },

  forgotPassword: async (email: string): Promise<{ message: string }> => {
    const { data } = await api.post<{ message: string }>('/auth/forgot-password', { email })
    return data
  },

  resetPassword: async (data: {
    token: string
    email: string
    password: string
    password_confirmation: string
  }): Promise<{ message: string }> => {
    const response = await api.post<{ message: string }>('/auth/reset-password', data)
    return response.data
  },

  updateProfile: async (data: { name: string; email: string }): Promise<User> => {
    const response = await api.put<User>('/user/profile', data)
    return response.data
  },

  updatePassword: async (data: {
    current_password: string
    password: string
    password_confirmation: string
  }): Promise<{ message: string }> => {
    const response = await api.put<{ message: string }>('/user/password', data)
    return response.data
  },

  deleteAccount: async (password: string): Promise<void> => {
    await api.delete('/user', { data: { password } })
    localStorage.removeItem('token')
  },
}

export default api
