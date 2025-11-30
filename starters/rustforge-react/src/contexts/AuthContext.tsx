import { createContext, useContext, useState, useEffect, ReactNode } from 'react'
import { User, authApi } from '../lib/api'

interface AuthContextType {
  user: User | null
  loading: boolean
  login: (email: string, password: string, remember?: boolean) => Promise<void>
  register: (name: string, email: string, password: string, passwordConfirmation: string) => Promise<void>
  logout: () => Promise<void>
  refreshUser: () => Promise<void>
}

const AuthContext = createContext<AuthContextType | undefined>(undefined)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    checkAuth()
  }, [])

  const checkAuth = async () => {
    const token = localStorage.getItem('token')
    if (!token) {
      setLoading(false)
      return
    }

    try {
      const userData = await authApi.getUser()
      setUser(userData)
    } catch {
      localStorage.removeItem('token')
    } finally {
      setLoading(false)
    }
  }

  const login = async (email: string, password: string, remember?: boolean) => {
    const { user } = await authApi.login({ email, password, remember })
    setUser(user)
  }

  const register = async (name: string, email: string, password: string, passwordConfirmation: string) => {
    const { user } = await authApi.register({
      name,
      email,
      password,
      password_confirmation: passwordConfirmation,
    })
    setUser(user)
  }

  const logout = async () => {
    await authApi.logout()
    setUser(null)
  }

  const refreshUser = async () => {
    const userData = await authApi.getUser()
    setUser(userData)
  }

  return (
    <AuthContext.Provider value={{ user, loading, login, register, logout, refreshUser }}>
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth() {
  const context = useContext(AuthContext)
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}
