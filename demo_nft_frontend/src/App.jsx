import { Fragment, useState } from "react";
import { Outlet } from "react-router-dom";
import UserWidget from "./UserWidget";
import { Dialog, Menu, Transition } from "@headlessui/react";
import {
  Bars3BottomLeftIcon,
  BellIcon,
  BuildingStorefrontIcon,
  ShoppingBagIcon,
  ChartBarIcon,
  HomeIcon,
  SparklesIcon,
  XMarkIcon,
} from "@heroicons/react/24/outline";
import { MagnifyingGlassIcon } from "@heroicons/react/20/solid";
import { classNames } from "./Utils";

export default function App() {
  return (
    <main>
      <UserWidget />
      <Outlet />
    </main>
  );
}
