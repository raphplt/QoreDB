import { ReactNode } from 'react';
import { Sidebar } from '../Sidebar/Sidebar';
import { TabBar } from '../Tabs/TabBar';
import './MainLayout.css';

interface MainLayoutProps {
  children: ReactNode;
}

export function MainLayout({ children }: MainLayoutProps) {
  return (
    <div className="layout">
      <Sidebar />
      <main className="layout-main">
        <TabBar />
        <div className="layout-content">
          {children}
        </div>
      </main>
    </div>
  );
}
