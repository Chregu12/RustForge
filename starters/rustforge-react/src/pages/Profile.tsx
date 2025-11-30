import { useState } from 'react'
import { useAuth } from '../contexts/AuthContext'
import { authApi } from '../lib/api'
import { Button } from '../components/ui/button'
import { Input } from '../components/ui/input'
import { Label } from '../components/ui/label'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'

export function Profile() {
  const { user, refreshUser } = useAuth()
  const [profileData, setProfileData] = useState({
    name: user?.name || '',
    email: user?.email || '',
  })
  const [passwordData, setPasswordData] = useState({
    current_password: '',
    password: '',
    password_confirmation: '',
  })
  const [profileStatus, setProfileStatus] = useState<string | null>(null)
  const [passwordStatus, setPasswordStatus] = useState<string | null>(null)
  const [profileLoading, setProfileLoading] = useState(false)
  const [passwordLoading, setPasswordLoading] = useState(false)

  const handleProfileUpdate = async (e: React.FormEvent) => {
    e.preventDefault()
    setProfileLoading(true)
    setProfileStatus(null)

    try {
      await authApi.updateProfile(profileData)
      await refreshUser()
      setProfileStatus('Profile updated successfully!')
    } catch (error: unknown) {
      const err = error as { response?: { data?: { message?: string } } }
      setProfileStatus(err.response?.data?.message || 'Failed to update profile')
    } finally {
      setProfileLoading(false)
    }
  }

  const handlePasswordUpdate = async (e: React.FormEvent) => {
    e.preventDefault()
    setPasswordLoading(true)
    setPasswordStatus(null)

    try {
      await authApi.updatePassword(passwordData)
      setPasswordData({
        current_password: '',
        password: '',
        password_confirmation: '',
      })
      setPasswordStatus('Password updated successfully!')
    } catch (error: unknown) {
      const err = error as { response?: { data?: { message?: string } } }
      setPasswordStatus(err.response?.data?.message || 'Failed to update password')
    } finally {
      setPasswordLoading(false)
    }
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Profile Information</CardTitle>
          <CardDescription>
            Update your account's profile information and email address.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleProfileUpdate} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="name">Name</Label>
              <Input
                id="name"
                value={profileData.name}
                onChange={(e) =>
                  setProfileData({ ...profileData, name: e.target.value })
                }
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="email">Email</Label>
              <Input
                id="email"
                type="email"
                value={profileData.email}
                onChange={(e) =>
                  setProfileData({ ...profileData, email: e.target.value })
                }
                required
              />
            </div>

            {profileStatus && (
              <p className={`text-sm ${profileStatus.includes('success') ? 'text-green-600' : 'text-red-600'}`}>
                {profileStatus}
              </p>
            )}

            <Button type="submit" disabled={profileLoading}>
              {profileLoading ? 'Saving...' : 'Save'}
            </Button>
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Update Password</CardTitle>
          <CardDescription>
            Ensure your account is using a long, random password to stay secure.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handlePasswordUpdate} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="current_password">Current Password</Label>
              <Input
                id="current_password"
                type="password"
                value={passwordData.current_password}
                onChange={(e) =>
                  setPasswordData({
                    ...passwordData,
                    current_password: e.target.value,
                  })
                }
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="password">New Password</Label>
              <Input
                id="password"
                type="password"
                value={passwordData.password}
                onChange={(e) =>
                  setPasswordData({ ...passwordData, password: e.target.value })
                }
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="password_confirmation">Confirm Password</Label>
              <Input
                id="password_confirmation"
                type="password"
                value={passwordData.password_confirmation}
                onChange={(e) =>
                  setPasswordData({
                    ...passwordData,
                    password_confirmation: e.target.value,
                  })
                }
                required
              />
            </div>

            {passwordStatus && (
              <p className={`text-sm ${passwordStatus.includes('success') ? 'text-green-600' : 'text-red-600'}`}>
                {passwordStatus}
              </p>
            )}

            <Button type="submit" disabled={passwordLoading}>
              {passwordLoading ? 'Saving...' : 'Save'}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}
