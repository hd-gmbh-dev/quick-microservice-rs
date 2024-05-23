# Keycloak realm roles definition table

This file will generate roles and groups in a keycloak realm based on markdown tables followed by `access_levels`, `user_groups` and `roles`


## User groups `user_groups`

| group            | name                 | display name         | access levels          | allowed types |
|------------------|----------------------|----------------------|------------------------|---------------|
| Admin            | /admin               | Admin                | Admin                  | none          |
| CustomerOwner    | /customer_owner      | Owner of Customer    | Customer               | none          |
| InstitutionOwner | /institution_owner   | Owner of Institution | Institution            | eco,state     |
| Management       | /management          | Management           | Customer, Institution  | eco           |
| Worker           | /worker              | Worker               | Institution            | state         |

## Roles `roles`

The administration is a special role in the example application, having access to all resources by default.

| Role                        | Admin | CustomerOwner | InstitutionOwner | Management | Worker |
|-----------------------------|-------|---------------|------------------|------------|--------|
| administration              | x     |               |                  |            |        |
| customer:list               |       | x             |                  |            |        |
| customer:view               |       | x             | x                | x          | x      |
| customer:update             |       | x             |                  |            |        |
| customer:create             |       | x             |                  |            |        |
| customer:delete             |       | x             |                  |            |        |
| customer:report             |       | x             |                  |            |        |
| institution:list            |       | x             | x                |            |        |
| institution:view            |       | x             | x                | x          | x      |
| institution:update          |       | x             | x                |            |        |
| institution:create          |       | x             | x                |            |        |
| institution:delete          |       | x             | x                |            |        |
| institution:report          |       | x             | x                |            |        |
| user:list                   |       | x             | x                | x          |        |
| user:view                   |       | x             | x                | x          |        |
| user:update                 |       | x             | x                | x          |        |
| user:create                 |       | x             | x                | x          |        |
| user:delete                 |       | x             | x                | x          |        |
| user:report                 |       | x             | x                | x          |        |
| employee:list               |       | x             | x                | x          | x      |
| employee:view               |       | x             | x                | x          | x      |
| employee:update             |       | x             | x                | x          |        |
| employee:create             |       | x             | x                | x          |        |
| employee:delete             |       | x             | x                | x          |        |
| employee:report             |       | x             | x                | x          |        |
| work_time:list              |       | x             | x                | x          | x      |
| work_time:view              |       | x             | x                | x          | x      |
| work_time:update            |       | x             | x                | x          | x      |
| work_time:create            |       | x             | x                | x          | x      |
| work_time:delete            |       | x             | x                | x          | x      |
| work_time:report            |       | x             | x                | x          | x      |
| employee_work_time:list     |       | x             | x                | x          |        |
| employee_work_time:view     |       | x             | x                | x          |        |
| employee_work_time:update   |       | x             | x                | x          |        |
| employee_work_time:create   |       | x             | x                | x          |        |
| employee_work_time:delete   |       | x             | x                | x          |        |
| employee_work_time:report   |       | x             | x                | x          |        |
| office:list                 |       | x             | x                | x          | x      |
| office:view                 |       | x             | x                | x          | x      |
| office:update               |       | x             | x                | x          |        |
| office:create               |       | x             | x                | x          |        |
| office:delete               |       | x             | x                | x          |        |
| office:report               |       | x             | x                | x          |        |
| appointment:list            |       | x             | x                | x          | x      |
| appointment:view            |       | x             | x                | x          | x      |
| appointment:update          |       | x             | x                | x          |        |
| appointment:create          |       | x             | x                | x          |        |
| appointment:delete          |       | x             | x                | x          |        |
| appointment:report          |       | x             | x                | x          |        |