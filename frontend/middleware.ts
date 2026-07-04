import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

/** Pass pathname to server layouts (admin return_to redirect). */
export function middleware(request: NextRequest) {
  const requestHeaders = new Headers(request.headers);
  const returnPath = `${request.nextUrl.pathname}${request.nextUrl.search}`;
  requestHeaders.set("x-pathname", returnPath);
  return NextResponse.next({
    request: { headers: requestHeaders },
  });
}

export const config = {
  matcher: ["/admin/:path*", "/login"],
};