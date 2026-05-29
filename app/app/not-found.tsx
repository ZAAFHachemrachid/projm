import Link from "next/link";

export default function NotFound() {
  return (
    <div className="flex flex-col items-center justify-center h-full gap-4">
      <h1 className="text-4xl font-bold">404</h1>
      <p className="text-muted-foreground">Page not found</p>
      <Link
        href="/"
        className="px-4 py-2 rounded-md bg-primary text-primary-foreground text-sm"
      >
        Back to Dashboard
      </Link>
    </div>
  );
}
