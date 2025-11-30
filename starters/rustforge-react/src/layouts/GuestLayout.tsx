import { Outlet, Link } from 'react-router-dom'

export function GuestLayout() {
  return (
    <div className="min-h-screen flex flex-col sm:justify-center items-center pt-6 sm:pt-0 bg-gray-100 dark:bg-gray-900">
      <div className="mb-6">
        <Link to="/" className="flex items-center gap-2">
          <svg
            className="w-12 h-12 text-primary"
            viewBox="0 0 24 24"
            fill="currentColor"
          >
            <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" />
          </svg>
          <span className="text-2xl font-bold text-gray-900 dark:text-white">
            RustForge
          </span>
        </Link>
      </div>

      <div className="w-full sm:max-w-md mt-6 px-6 py-4 bg-white dark:bg-gray-800 shadow-md overflow-hidden sm:rounded-lg">
        <Outlet />
      </div>
    </div>
  )
}
