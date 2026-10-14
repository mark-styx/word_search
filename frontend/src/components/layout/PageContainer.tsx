"use client";

import { ReactNode } from "react";

interface PageContainerProps {
  children: ReactNode;
  className?: string;
}

export default function PageContainer({ children, className = "" }: PageContainerProps) {
  return (
    <main className={`mx-auto w-full max-w-5xl flex-1 px-4 py-6 pb-[calc(1.5rem+env(safe-area-inset-bottom))] ${className}`}>
      {children}
    </main>
  );
}
