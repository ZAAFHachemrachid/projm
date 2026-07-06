import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { Titlebar } from "@/components/titlebar";
import { UpdateChecker } from "@/components/update-checker";
import { themeInitScript } from "@/lib/themes";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable} h-full antialiased dark`}
    >
      <head>
        {/* Apply the saved theme before first paint to avoid a flash. */}
        <script dangerouslySetInnerHTML={{ __html: themeInitScript() }} />
      </head>
      <body className="h-full bg-background text-foreground overflow-hidden flex flex-col">
        <Titlebar />
        <UpdateChecker />
        <div className="flex-1 min-h-0">
          {children}
        </div>
      </body>
    </html>
  );
}
