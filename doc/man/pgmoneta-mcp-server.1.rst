===================
pgmoneta-mcp-server
===================

--------------------------------------------------------------------------------------
A Model Context Protocol (MCP) server for pgmoneta, backup/restore tool for PostgreSQL
--------------------------------------------------------------------------------------

:Manual section: 1

SYNOPSIS
========

pgmoneta-mcp-server [ OPTIONS ]

DESCRIPTION
===========

pgmoneta-mcp-server is an MCP server for pgmoneta, providing a natural language interface for PostgreSQL backup information.

OPTIONS
=======

-c, --conf CONF
  Path to pgmoneta MCP configuration file (default: /etc/pgmoneta-mcp/pgmoneta-mcp.conf)

-u, --users USERS
  Path to pgmoneta MCP users configuration file (default: /etc/pgmoneta-mcp/pgmoneta-mcp-users.conf)

-h, --help
  Print help

-V, --version
  Print version

REPORTING BUGS
==============

pgmoneta-mcp is maintained on GitHub at https://github.com/pgmoneta/pgmoneta-mcp

COPYRIGHT
=========

pgmoneta-mcp is licensed under the GNU General Public License v3.0

SEE ALSO
========

pgmoneta-mcp-admin(1)
