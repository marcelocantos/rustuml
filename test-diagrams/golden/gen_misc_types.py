#!/usr/bin/env python3
"""
Generator for comprehensive PlantUML test cases covering:
- ER diagrams (entity-relationship using class notation)
- ArchiMate diagrams
- Ditaa diagrams
- Regex diagrams
- Math/LaTeX diagrams
- Combined/kitchen-sink diagrams
- Sprite and icon tests
- Link/URL tests
"""

import os
import itertools

BASE = os.path.dirname(os.path.abspath(__file__))

DIRS = ["er", "archimate", "wire", "regex", "math", "combo", "sprites", "links"]

for d in DIRS:
    os.makedirs(os.path.join(BASE, d), exist_ok=True)


def write(subdir, filename, content):
    path = os.path.join(BASE, subdir, filename)
    with open(path, "w") as f:
        f.write(content.strip() + "\n")


# ==============================================================================
# ER DIAGRAMS
# ==============================================================================

def gen_er():
    # 1. Basic one-to-many
    write("er", "er_one_to_many.puml", """
@startuml
entity Customer {
  * id : INT <<PK>>
  --
  name : VARCHAR(100)
  email : VARCHAR(200)
  created_at : DATETIME
}
entity Order {
  * id : INT <<PK>>
  --
  * customer_id : INT <<FK>>
  total : DECIMAL(10,2)
  status : VARCHAR(20)
  order_date : DATE
}
Customer ||--o{ Order : "places"
@enduml
""")

    # 2. Many-to-many with join table
    write("er", "er_many_to_many.puml", """
@startuml
entity Student {
  * id : INT <<PK>>
  --
  name : VARCHAR(100)
  email : VARCHAR(200)
}
entity Course {
  * id : INT <<PK>>
  --
  title : VARCHAR(200)
  credits : INT
}
entity Enrollment {
  * student_id : INT <<FK>>
  * course_id : INT <<FK>>
  --
  enrolled_at : DATETIME
  grade : CHAR(2)
}
Student ||--o{ Enrollment : "enrolls in"
Course ||--o{ Enrollment : "has"
@enduml
""")

    # 3. One-to-one
    write("er", "er_one_to_one.puml", """
@startuml
entity User {
  * id : INT <<PK>>
  --
  username : VARCHAR(50)
  password_hash : CHAR(64)
}
entity UserProfile {
  * id : INT <<PK>>
  * user_id : INT <<FK>>
  --
  full_name : VARCHAR(200)
  bio : TEXT
  avatar_url : VARCHAR(500)
}
User ||--|| UserProfile : "has"
@enduml
""")

    # 4. Self-referencing
    write("er", "er_self_reference.puml", """
@startuml
entity Employee {
  * id : INT <<PK>>
  --
  name : VARCHAR(100)
  manager_id : INT <<FK>>
  department : VARCHAR(50)
  salary : DECIMAL(10,2)
}
Employee ||--o{ Employee : "manages"
@enduml
""")

    # 5. Weak entity
    write("er", "er_weak_entity.puml", """
@startuml
entity Order {
  * id : INT <<PK>>
  --
  customer_id : INT <<FK>>
  order_date : DATE
}
entity OrderItem {
  * order_id : INT <<PK,FK>>
  * line_num : INT <<PK>>
  --
  product_id : INT <<FK>>
  quantity : INT
  unit_price : DECIMAL(10,2)
}
entity Product {
  * id : INT <<PK>>
  --
  name : VARCHAR(200)
  price : DECIMAL(10,2)
}
Order ||--o{ OrderItem : "contains"
Product ||--o{ OrderItem : "included in"
@enduml
""")

    # 6. Complex schema: e-commerce
    write("er", "er_ecommerce_schema.puml", """
@startuml
skinparam linetype ortho

package "Users" {
  entity User {
    * id : INT <<PK>>
    --
    email : VARCHAR(200) UNIQUE
    password_hash : CHAR(64)
    created_at : TIMESTAMP
    is_active : BOOLEAN
  }
  entity Address {
    * id : INT <<PK>>
    * user_id : INT <<FK>>
    --
    street : VARCHAR(200)
    city : VARCHAR(100)
    state : VARCHAR(50)
    zip : VARCHAR(20)
    country : CHAR(2)
    is_default : BOOLEAN
  }
}

package "Catalog" {
  entity Category {
    * id : INT <<PK>>
    --
    name : VARCHAR(100)
    parent_id : INT <<FK>>
    slug : VARCHAR(100)
  }
  entity Product {
    * id : INT <<PK>>
    * category_id : INT <<FK>>
    --
    sku : VARCHAR(50) UNIQUE
    name : VARCHAR(200)
    description : TEXT
    price : DECIMAL(10,2)
    stock : INT
    weight_kg : DECIMAL(6,3)
  }
  entity ProductImage {
    * id : INT <<PK>>
    * product_id : INT <<FK>>
    --
    url : VARCHAR(500)
    alt_text : VARCHAR(200)
    sort_order : INT
  }
}

package "Orders" {
  entity Cart {
    * id : INT <<PK>>
    * user_id : INT <<FK>>
    --
    created_at : TIMESTAMP
    updated_at : TIMESTAMP
  }
  entity CartItem {
    * cart_id : INT <<FK>>
    * product_id : INT <<FK>>
    --
    quantity : INT
    added_at : TIMESTAMP
  }
  entity Order {
    * id : INT <<PK>>
    * user_id : INT <<FK>>
    * shipping_address_id : INT <<FK>>
    --
    status : ENUM
    subtotal : DECIMAL(10,2)
    shipping : DECIMAL(10,2)
    tax : DECIMAL(10,2)
    total : DECIMAL(10,2)
    placed_at : TIMESTAMP
  }
  entity OrderItem {
    * order_id : INT <<FK>>
    * product_id : INT <<FK>>
    --
    quantity : INT
    unit_price : DECIMAL(10,2)
    line_total : DECIMAL(10,2)
  }
}

User ||--o{ Address : "has"
User ||--o{ Cart : "has"
User ||--o{ Order : "places"
Category ||--o{ Category : "parent of"
Category ||--o{ Product : "contains"
Product ||--o{ ProductImage : "has"
Cart ||--o{ CartItem : "contains"
CartItem }o--|| Product : "references"
Order ||--o{ OrderItem : "contains"
OrderItem }o--|| Product : "includes"
Order }o--|| Address : "ships to"
@enduml
""")

    # 7. All crow's foot notations
    write("er", "er_crowsfoot_all.puml", """
@startuml
note "Crow's Foot Notation Examples" as N

entity A {
  * id : INT <<PK>>
}
entity B {
  * id : INT <<PK>>
}
entity C {
  * id : INT <<PK>>
}
entity D {
  * id : INT <<PK>>
}
entity E {
  * id : INT <<PK>>
}
entity F {
  * id : INT <<PK>>
}

A ||--|| B : "one to one"
A ||--o| C : "one to zero-or-one"
A ||--|{ D : "one to one-or-many"
A ||--o{ E : "one to zero-or-many"
A }|--|{ F : "many-or-one to many-or-one"
@enduml
""")

    # 8. IE notation variants (left side)
    write("er", "er_ie_notation_left.puml", """
@startuml
entity Foo {
  * id : INT <<PK>>
}
entity Bar {
  * id : INT <<PK>>
}
entity Baz {
  * id : INT <<PK>>
}
entity Qux {
  * id : INT <<PK>>
}
entity Quux {
  * id : INT <<PK>>
}

Foo }o--|| Bar : "zero-or-many to one"
Foo }|--|| Baz : "one-or-many to one"
Foo o|--|| Qux : "zero-or-one to one"
Foo ||--|| Quux : "one to one"
@enduml
""")

    # 9. Ternary relationship (simulated)
    write("er", "er_ternary.puml", """
@startuml
entity Doctor {
  * id : INT <<PK>>
  --
  name : VARCHAR(100)
  specialty : VARCHAR(100)
}
entity Patient {
  * id : INT <<PK>>
  --
  name : VARCHAR(100)
  dob : DATE
}
entity Hospital {
  * id : INT <<PK>>
  --
  name : VARCHAR(200)
  city : VARCHAR(100)
}
entity Appointment {
  * doctor_id : INT <<FK>>
  * patient_id : INT <<FK>>
  * hospital_id : INT <<FK>>
  --
  scheduled_at : DATETIME
  duration_min : INT
  notes : TEXT
}
Doctor ||--o{ Appointment : "attends"
Patient ||--o{ Appointment : "has"
Hospital ||--o{ Appointment : "hosts"
@enduml
""")

    # 10. Entity with many fields (15+)
    write("er", "er_many_fields.puml", """
@startuml
entity Person {
  * id : BIGINT <<PK>>
  --
  first_name : VARCHAR(50)
  middle_name : VARCHAR(50)
  last_name : VARCHAR(50)
  preferred_name : VARCHAR(100)
  date_of_birth : DATE
  gender : CHAR(1)
  nationality : CHAR(2)
  passport_number : VARCHAR(20)
  ssn : CHAR(11)
  phone_mobile : VARCHAR(20)
  phone_home : VARCHAR(20)
  phone_work : VARCHAR(20)
  email_primary : VARCHAR(200)
  email_secondary : VARCHAR(200)
  address_line1 : VARCHAR(200)
  address_line2 : VARCHAR(200)
  city : VARCHAR(100)
  state : VARCHAR(50)
  zip : VARCHAR(20)
  country : CHAR(2)
  created_at : TIMESTAMP
  updated_at : TIMESTAMP
  is_active : BOOLEAN
}
@enduml
""")

    # 11. HR schema
    write("er", "er_hr_schema.puml", """
@startuml
skinparam linetype ortho

entity Department {
  * id : INT <<PK>>
  --
  name : VARCHAR(100)
  cost_center : VARCHAR(20)
  manager_id : INT <<FK>>
}
entity Employee {
  * id : INT <<PK>>
  * dept_id : INT <<FK>>
  --
  name : VARCHAR(100)
  title : VARCHAR(100)
  hire_date : DATE
  salary : DECIMAL(12,2)
  manager_id : INT <<FK>>
}
entity JobHistory {
  * employee_id : INT <<FK>>
  * start_date : DATE
  --
  end_date : DATE
  job_title : VARCHAR(100)
  dept_id : INT <<FK>>
  salary : DECIMAL(12,2)
}
entity Skill {
  * id : INT <<PK>>
  --
  name : VARCHAR(100)
  category : VARCHAR(50)
}
entity EmployeeSkill {
  * employee_id : INT <<FK>>
  * skill_id : INT <<FK>>
  --
  proficiency : INT
  certified : BOOLEAN
  cert_date : DATE
}
entity Benefit {
  * id : INT <<PK>>
  --
  name : VARCHAR(100)
  type : VARCHAR(50)
  annual_cost : DECIMAL(10,2)
}
entity EmployeeBenefit {
  * employee_id : INT <<FK>>
  * benefit_id : INT <<FK>>
  --
  enrolled_at : DATE
}
Department ||--o{ Employee : "employs"
Employee ||--o{ Employee : "manages"
Department ||--o{ Department : "sub-dept"
Employee ||--o{ JobHistory : "has"
Employee ||--o{ EmployeeSkill : "has"
Skill ||--o{ EmployeeSkill : "used by"
Employee ||--o{ EmployeeBenefit : "enrolled in"
Benefit ||--o{ EmployeeBenefit : "includes"
@enduml
""")

    # 12. Library system
    write("er", "er_library_system.puml", """
@startuml
entity Book {
  * isbn : CHAR(13) <<PK>>
  --
  title : VARCHAR(300)
  subtitle : VARCHAR(300)
  publisher_id : INT <<FK>>
  pub_year : INT
  edition : INT
  pages : INT
  language : CHAR(3)
}
entity Author {
  * id : INT <<PK>>
  --
  first_name : VARCHAR(100)
  last_name : VARCHAR(100)
  bio : TEXT
}
entity BookAuthor {
  * isbn : CHAR(13) <<FK>>
  * author_id : INT <<FK>>
  --
  role : VARCHAR(50)
  author_order : INT
}
entity Publisher {
  * id : INT <<PK>>
  --
  name : VARCHAR(200)
  country : CHAR(2)
  website : VARCHAR(300)
}
entity Copy {
  * id : INT <<PK>>
  * isbn : CHAR(13) <<FK>>
  --
  barcode : VARCHAR(50) UNIQUE
  condition : VARCHAR(20)
  acquired_at : DATE
  is_available : BOOLEAN
}
entity Member {
  * id : INT <<PK>>
  --
  name : VARCHAR(200)
  email : VARCHAR(200)
  phone : VARCHAR(20)
  membership_expires : DATE
}
entity Loan {
  * id : INT <<PK>>
  * copy_id : INT <<FK>>
  * member_id : INT <<FK>>
  --
  loaned_at : DATETIME
  due_date : DATE
  returned_at : DATETIME
  fine_amount : DECIMAL(6,2)
}
Book }o--|| Publisher : "published by"
Book ||--o{ BookAuthor : "written by"
Author ||--o{ BookAuthor : "writes"
Book ||--o{ Copy : "has"
Copy ||--o{ Loan : "loaned in"
Member ||--o{ Loan : "borrows"
@enduml
""")

    # 13. Zero-or-one relationships
    write("er", "er_optional_relationships.puml", """
@startuml
entity Person {
  * id : INT <<PK>>
  --
  name : VARCHAR(100)
}
entity Passport {
  * id : INT <<PK>>
  * person_id : INT <<FK>>
  --
  number : VARCHAR(20)
  issued_by : CHAR(2)
  expires : DATE
}
entity DriverLicense {
  * id : INT <<PK>>
  * person_id : INT <<FK>>
  --
  number : VARCHAR(20)
  state : VARCHAR(50)
  class : CHAR(1)
  expires : DATE
}
entity VehicleRegistration {
  * id : INT <<PK>>
  * owner_id : INT <<FK>>
  --
  plate : VARCHAR(10)
  make : VARCHAR(50)
  model : VARCHAR(50)
  year : INT
  vin : CHAR(17)
}
Person ||--o| Passport : "may have"
Person ||--o| DriverLicense : "may have"
Person ||--o{ VehicleRegistration : "may own"
@enduml
""")

    # 14. Financial schema
    write("er", "er_financial_schema.puml", """
@startuml
skinparam linetype ortho

entity Account {
  * id : BIGINT <<PK>>
  --
  number : VARCHAR(20) UNIQUE
  type : ENUM
  balance : DECIMAL(15,2)
  currency : CHAR(3)
  opened_at : DATE
  closed_at : DATE
  owner_id : INT <<FK>>
}
entity Customer {
  * id : INT <<PK>>
  --
  name : VARCHAR(200)
  tax_id : VARCHAR(20)
  kyc_status : VARCHAR(20)
}
entity Transaction {
  * id : BIGINT <<PK>>
  * account_id : BIGINT <<FK>>
  --
  type : ENUM
  amount : DECIMAL(15,2)
  currency : CHAR(3)
  description : VARCHAR(300)
  reference : VARCHAR(50)
  executed_at : TIMESTAMP
  settled_at : TIMESTAMP
  status : VARCHAR(20)
}
entity Transfer {
  * id : BIGINT <<PK>>
  * from_account_id : BIGINT <<FK>>
  * to_account_id : BIGINT <<FK>>
  --
  amount : DECIMAL(15,2)
  fee : DECIMAL(10,2)
  fx_rate : DECIMAL(10,6)
  initiated_at : TIMESTAMP
  completed_at : TIMESTAMP
}
entity Beneficiary {
  * id : INT <<PK>>
  * customer_id : INT <<FK>>
  --
  name : VARCHAR(200)
  account_number : VARCHAR(30)
  bank_code : VARCHAR(20)
  country : CHAR(2)
}
Customer ||--o{ Account : "owns"
Account ||--o{ Transaction : "records"
Account ||--o{ Transfer : "sends from"
Account ||--o{ Transfer : "receives to"
Customer ||--o{ Beneficiary : "registers"
@enduml
""")

    # 15. Healthcare schema
    write("er", "er_healthcare_schema.puml", """
@startuml
skinparam linetype ortho

entity Patient {
  * id : INT <<PK>>
  --
  mrn : VARCHAR(20) UNIQUE
  first_name : VARCHAR(50)
  last_name : VARCHAR(50)
  dob : DATE
  sex : CHAR(1)
  blood_type : VARCHAR(3)
  allergies : TEXT
}
entity Provider {
  * id : INT <<PK>>
  --
  npi : CHAR(10) UNIQUE
  name : VARCHAR(200)
  specialty : VARCHAR(100)
  license_state : CHAR(2)
  license_number : VARCHAR(20)
}
entity Facility {
  * id : INT <<PK>>
  --
  name : VARCHAR(200)
  address : VARCHAR(300)
  npi : CHAR(10) UNIQUE
  type : VARCHAR(50)
}
entity Visit {
  * id : INT <<PK>>
  * patient_id : INT <<FK>>
  * provider_id : INT <<FK>>
  * facility_id : INT <<FK>>
  --
  visit_date : DATE
  chief_complaint : VARCHAR(500)
  diagnosis_codes : VARCHAR(200)
  notes : TEXT
}
entity Prescription {
  * id : INT <<PK>>
  * visit_id : INT <<FK>>
  * prescriber_id : INT <<FK>>
  --
  drug_ndc : VARCHAR(20)
  drug_name : VARCHAR(200)
  dosage : VARCHAR(100)
  frequency : VARCHAR(100)
  quantity : INT
  refills : INT
  written_at : DATETIME
}
entity LabResult {
  * id : INT <<PK>>
  * visit_id : INT <<FK>>
  --
  test_code : VARCHAR(20)
  test_name : VARCHAR(200)
  result_value : VARCHAR(100)
  unit : VARCHAR(50)
  reference_range : VARCHAR(100)
  abnormal_flag : CHAR(1)
  collected_at : DATETIME
  resulted_at : DATETIME
}
Patient ||--o{ Visit : "has"
Provider ||--o{ Visit : "conducts"
Facility ||--o{ Visit : "hosts"
Visit ||--o{ Prescription : "generates"
Visit ||--o{ LabResult : "produces"
Provider ||--o{ Prescription : "writes"
@enduml
""")

    # 16-25: More ER variations
    for i, (name, content) in enumerate([
        ("er_zero_or_many", """
@startuml
entity Parent {
  * id : INT <<PK>>
  --
  name : VARCHAR(100)
}
entity Child {
  * id : INT <<PK>>
  * parent_id : INT <<FK>>
  --
  name : VARCHAR(100)
  age : INT
}
Parent ||--o{ Child : "has"
@enduml
"""),
        ("er_exactly_one", """
@startuml
entity Contract {
  * id : INT <<PK>>
  --
  title : VARCHAR(200)
  start_date : DATE
  end_date : DATE
}
entity ContractPartyA {
  * id : INT <<PK>>
  * contract_id : INT <<FK>>
  --
  name : VARCHAR(200)
  role : VARCHAR(100)
}
entity ContractPartyB {
  * id : INT <<PK>>
  * contract_id : INT <<FK>>
  --
  name : VARCHAR(200)
  role : VARCHAR(100)
}
Contract ||--|| ContractPartyA : "party A"
Contract ||--|| ContractPartyB : "party B"
@enduml
"""),
        ("er_one_or_many", """
@startuml
entity Team {
  * id : INT <<PK>>
  --
  name : VARCHAR(100)
  created_at : DATE
}
entity TeamMember {
  * id : INT <<PK>>
  * team_id : INT <<FK>>
  --
  user_id : INT <<FK>>
  role : VARCHAR(50)
  joined_at : DATE
}
Team ||--|{ TeamMember : "has"
@enduml
"""),
        ("er_blog_schema", """
@startuml
entity Author {
  * id : INT <<PK>>
  --
  username : VARCHAR(50)
  email : VARCHAR(200)
  bio : TEXT
}
entity Post {
  * id : INT <<PK>>
  * author_id : INT <<FK>>
  --
  title : VARCHAR(300)
  slug : VARCHAR(300)
  body : TEXT
  published_at : DATETIME
  is_draft : BOOLEAN
}
entity Tag {
  * id : INT <<PK>>
  --
  name : VARCHAR(50)
  slug : VARCHAR(50)
}
entity PostTag {
  * post_id : INT <<FK>>
  * tag_id : INT <<FK>>
}
entity Comment {
  * id : INT <<PK>>
  * post_id : INT <<FK>>
  --
  author_name : VARCHAR(100)
  author_email : VARCHAR(200)
  body : TEXT
  posted_at : DATETIME
  parent_id : INT <<FK>>
}
Author ||--o{ Post : "writes"
Post ||--o{ PostTag : "tagged with"
Tag ||--o{ PostTag : "applied to"
Post ||--o{ Comment : "receives"
Comment ||--o{ Comment : "replied to"
@enduml
"""),
        ("er_inventory_schema", """
@startuml
entity Warehouse {
  * id : INT <<PK>>
  --
  name : VARCHAR(100)
  location : VARCHAR(200)
  capacity : INT
}
entity SKU {
  * id : INT <<PK>>
  --
  code : VARCHAR(50) UNIQUE
  name : VARCHAR(200)
  unit : VARCHAR(20)
  reorder_point : INT
}
entity Stock {
  * warehouse_id : INT <<FK>>
  * sku_id : INT <<FK>>
  --
  quantity : INT
  reserved : INT
  bin_location : VARCHAR(20)
  last_counted : DATE
}
entity PurchaseOrder {
  * id : INT <<PK>>
  * warehouse_id : INT <<FK>>
  --
  supplier_id : INT <<FK>>
  status : VARCHAR(20)
  ordered_at : DATE
  expected_at : DATE
}
entity POLine {
  * po_id : INT <<FK>>
  * sku_id : INT <<FK>>
  --
  ordered_qty : INT
  received_qty : INT
  unit_cost : DECIMAL(10,2)
}
Warehouse ||--o{ Stock : "holds"
SKU ||--o{ Stock : "stored as"
Warehouse ||--o{ PurchaseOrder : "receives"
PurchaseOrder ||--o{ POLine : "contains"
SKU ||--o{ POLine : "ordered as"
@enduml
"""),
        ("er_social_network", """
@startuml
entity User {
  * id : BIGINT <<PK>>
  --
  handle : VARCHAR(50) UNIQUE
  display_name : VARCHAR(100)
  bio : VARCHAR(500)
  created_at : TIMESTAMP
}
entity Follow {
  * follower_id : BIGINT <<FK>>
  * followed_id : BIGINT <<FK>>
  --
  created_at : TIMESTAMP
}
entity Post {
  * id : BIGINT <<PK>>
  * user_id : BIGINT <<FK>>
  --
  content : TEXT
  media_url : VARCHAR(500)
  created_at : TIMESTAMP
}
entity Like {
  * user_id : BIGINT <<FK>>
  * post_id : BIGINT <<FK>>
  --
  created_at : TIMESTAMP
}
entity Message {
  * id : BIGINT <<PK>>
  * sender_id : BIGINT <<FK>>
  * recipient_id : BIGINT <<FK>>
  --
  content : TEXT
  sent_at : TIMESTAMP
  read_at : TIMESTAMP
}
User ||--o{ Follow : "following"
User ||--o{ Follow : "followed by"
User ||--o{ Post : "creates"
User ||--o{ Like : "gives"
Post ||--o{ Like : "receives"
User ||--o{ Message : "sends"
User ||--o{ Message : "receives"
@enduml
"""),
        ("er_cms_schema", """
@startuml
entity Site {
  * id : INT <<PK>>
  --
  domain : VARCHAR(200)
  title : VARCHAR(200)
  theme : VARCHAR(50)
}
entity Page {
  * id : INT <<PK>>
  * site_id : INT <<FK>>
  --
  slug : VARCHAR(200)
  title : VARCHAR(300)
  content : LONGTEXT
  template : VARCHAR(50)
  is_published : BOOLEAN
  published_at : DATETIME
  parent_id : INT <<FK>>
}
entity Media {
  * id : INT <<PK>>
  * site_id : INT <<FK>>
  --
  filename : VARCHAR(200)
  mime_type : VARCHAR(100)
  size_bytes : BIGINT
  url : VARCHAR(500)
  alt_text : VARCHAR(300)
  uploaded_at : DATETIME
}
entity Menu {
  * id : INT <<PK>>
  * site_id : INT <<FK>>
  --
  name : VARCHAR(100)
  location : VARCHAR(50)
}
entity MenuItem {
  * id : INT <<PK>>
  * menu_id : INT <<FK>>
  --
  label : VARCHAR(100)
  url : VARCHAR(500)
  page_id : INT <<FK>>
  sort_order : INT
  parent_id : INT <<FK>>
}
Site ||--o{ Page : "contains"
Site ||--o{ Media : "stores"
Site ||--o{ Menu : "has"
Menu ||--o{ MenuItem : "contains"
Page ||--o{ MenuItem : "linked from"
Page ||--o{ Page : "child of"
MenuItem ||--o{ MenuItem : "child of"
@enduml
"""),
        ("er_project_management", """
@startuml
entity Project {
  * id : INT <<PK>>
  --
  name : VARCHAR(200)
  description : TEXT
  status : VARCHAR(20)
  start_date : DATE
  end_date : DATE
  budget : DECIMAL(12,2)
}
entity Milestone {
  * id : INT <<PK>>
  * project_id : INT <<FK>>
  --
  name : VARCHAR(200)
  due_date : DATE
  is_completed : BOOLEAN
}
entity Task {
  * id : INT <<PK>>
  * project_id : INT <<FK>>
  * milestone_id : INT <<FK>>
  --
  title : VARCHAR(300)
  description : TEXT
  status : VARCHAR(20)
  priority : INT
  estimated_hours : DECIMAL(6,1)
  actual_hours : DECIMAL(6,1)
  assigned_to : INT <<FK>>
  created_at : DATETIME
  due_date : DATE
  parent_task_id : INT <<FK>>
}
entity TeamMember {
  * id : INT <<PK>>
  * project_id : INT <<FK>>
  --
  user_id : INT <<FK>>
  role : VARCHAR(50)
  joined_at : DATE
}
entity TimeLog {
  * id : INT <<PK>>
  * task_id : INT <<FK>>
  * user_id : INT <<FK>>
  --
  hours : DECIMAL(5,2)
  description : VARCHAR(500)
  logged_at : DATE
}
Project ||--o{ Milestone : "has"
Project ||--o{ Task : "contains"
Milestone ||--o{ Task : "groups"
Task ||--o{ Task : "subtask of"
Project ||--o{ TeamMember : "has"
Task ||--o{ TimeLog : "tracks"
@enduml
"""),
        ("er_school_schema", """
@startuml
entity School {
  * id : INT <<PK>>
  --
  name : VARCHAR(200)
  address : VARCHAR(300)
  principal : VARCHAR(200)
}
entity Class {
  * id : INT <<PK>>
  * school_id : INT <<FK>>
  --
  name : VARCHAR(50)
  grade_level : INT
  room : VARCHAR(20)
  year : INT
}
entity Teacher {
  * id : INT <<PK>>
  * school_id : INT <<FK>>
  --
  name : VARCHAR(200)
  email : VARCHAR(200)
  subjects : VARCHAR(300)
}
entity Student {
  * id : INT <<PK>>
  * school_id : INT <<FK>>
  * class_id : INT <<FK>>
  --
  name : VARCHAR(200)
  dob : DATE
  guardian_name : VARCHAR(200)
  guardian_phone : VARCHAR(20)
}
entity Subject {
  * id : INT <<PK>>
  --
  name : VARCHAR(100)
  code : VARCHAR(20)
  credit_hours : INT
}
entity ClassSubject {
  * class_id : INT <<FK>>
  * subject_id : INT <<FK>>
  * teacher_id : INT <<FK>>
  --
  schedule : VARCHAR(200)
}
entity Grade {
  * id : INT <<PK>>
  * student_id : INT <<FK>>
  * subject_id : INT <<FK>>
  --
  term : VARCHAR(20)
  score : DECIMAL(5,2)
  letter_grade : CHAR(2)
}
School ||--o{ Class : "has"
School ||--o{ Teacher : "employs"
Class ||--o{ Student : "enrolls"
Class ||--o{ ClassSubject : "teaches"
Subject ||--o{ ClassSubject : "covered by"
Teacher ||--o{ ClassSubject : "teaches"
Student ||--o{ Grade : "receives"
Subject ||--o{ Grade : "graded in"
@enduml
"""),
    ], start=16):
        fname = name + ".puml"
        write("er", fname, content)

    # 26-50: More ER patterns
    for i in range(26, 51):
        fk_count = (i % 4) + 2
        entity_count = (i % 5) + 3
        entities = [f"Entity{chr(65+j)}" for j in range(entity_count)]
        lines = ["@startuml"]
        for e in entities:
            lines.append(f"entity {e} {{")
            lines.append(f"  * id : INT <<PK>>")
            lines.append(f"  --")
            lines.append(f"  name : VARCHAR(100)")
            lines.append(f"  code : VARCHAR(20)")
            lines.append(f"  created_at : DATETIME")
            lines.append(f"}}")
        notations = ["||--||", "||--o{", "}|--|{", "||--|{", "}o--||", "o|--||"]
        for j in range(min(fk_count, entity_count-1)):
            e1 = entities[j]
            e2 = entities[j+1]
            notation = notations[j % len(notations)]
            lines.append(f'{e1} {notation} {e2} : "rel_{j+1}"')
        lines.append("@enduml")
        write("er", f"er_pattern_{i:03d}.puml", "\n".join(lines))

    # 51-100: Schema variations with packages and multiple tables
    for i in range(51, 101):
        table_count = (i % 6) + 4
        write("er", f"er_schema_variant_{i:03d}.puml", f"""
@startuml
skinparam linetype ortho

package "Schema{i}" {{
{chr(10).join(f'''  entity Table{j} {{
    * id : INT <<PK>>
    --
    name : VARCHAR(100)
    value : DECIMAL(10,2)
    active : BOOLEAN
    created_at : TIMESTAMP
  }}''' for j in range(1, table_count+1))}
}}

{chr(10).join(f"Table{j} ||--o{{ Table{j+1} : \"ref\"" for j in range(1, table_count))}
@enduml
""")

    # 101-150: IE notation exhaustive tests
    crowsfoot_pairs = [
        ("||", "||"), ("||", "o|"), ("||", "|{"), ("||", "o{"),
        ("o|", "||"), ("o|", "o|"), ("o|", "|{"), ("o|", "o{"),
        ("|{", "||"), ("|{", "o|"), ("|{", "|{"), ("|{", "o{"),
        ("o{", "||"), ("o{", "o|"), ("o{", "|{"), ("o{", "o{"),
    ]
    for idx, (left, right) in enumerate(crowsfoot_pairs[:50]):
        write("er", f"er_ie_{idx+101:03d}.puml", f"""
@startuml
entity Left {{
  * id : INT <<PK>>
  --
  name : VARCHAR(100)
}}
entity Right {{
  * id : INT <<PK>>
  --
  name : VARCHAR(100)
}}
Left {left}--{right} Right : "relationship"
@enduml
""")

gen_er()


# ==============================================================================
# ARCHIMATE DIAGRAMS
# ==============================================================================

def gen_archimate():
    # 1. Basic motivation
    write("archimate", "archimate_basic.puml", """
@startuml
!include <archimate/Archimate>

Motivation_Stakeholder(stakeholder, "Customer")
Motivation_Goal(goal, "Reduce Costs")
Motivation_Requirement(req, "24h Availability")

Rel_Association_Up(stakeholder, goal, "has")
Rel_Realization_Up(req, goal, "realizes")
@enduml
""")

    # 2. Business layer
    write("archimate", "archimate_business.puml", """
@startuml
!include <archimate/Archimate>

Business_Actor(cust, "Customer")
Business_Actor(agent, "Sales Agent")
Business_Role(mgr, "Account Manager")
Business_Process(sale, "Sales Process")
Business_Service(svc, "Order Service")
Business_Object(order, "Order")

Rel_Assignment(agent, mgr, "")
Rel_Association(cust, sale, "participates")
Rel_Serving(svc, sale, "supports")
Rel_Access_Write(sale, order, "creates")
@enduml
""")

    # 3. Application layer
    write("archimate", "archimate_application.puml", """
@startuml
!include <archimate/Archimate>

Application_Component(crm, "CRM System")
Application_Component(erp, "ERP System")
Application_Interface(api, "REST API")
Application_Service(ordersvc, "Order Service")
Application_DataObject(orderdata, "Order Data")

Rel_Composition(crm, api, "exposes")
Rel_Serving(ordersvc, erp, "uses")
Rel_Access_ReadWrite(ordersvc, orderdata, "manages")
Rel_Association(crm, erp, "integrates with")
@enduml
""")

    # 4. Technology layer
    write("archimate", "archimate_technology.puml", """
@startuml
!include <archimate/Archimate>

Technology_Node(server, "App Server")
Technology_Node(db, "Database Server")
Technology_Artifact(jar, "app.jar")
Technology_SystemSoftware(jvm, "JVM 17")
Technology_CommunicationNetwork(net, "Internal Network")

Rel_Composition(server, jvm, "hosts")
Rel_Composition(server, jar, "runs")
Rel_Association(server, db, "connects via")
Rel_Association(server, net, "uses")
@enduml
""")

    # 5. Full stack (all layers)
    write("archimate", "archimate_full_stack.puml", """
@startuml
!include <archimate/Archimate>

skinparam rectangle {
  BackgroundColor #EEEEEE
}

rectangle "Business Layer" {
  Business_Actor(actor, "End User")
  Business_Process(proc, "Order Fulfillment")
}

rectangle "Application Layer" {
  Application_Component(app, "Web App")
  Application_Service(svc, "Fulfillment Service")
}

rectangle "Technology Layer" {
  Technology_Node(node, "Linux Server")
  Technology_SystemSoftware(db, "PostgreSQL")
}

Rel_Serving(app, actor, "serves")
Rel_Realization(svc, proc, "realizes")
Rel_Serving(node, app, "hosts")
Rel_Association(node, db, "uses")
@enduml
""")

    # 6. Migration view
    write("archimate", "archimate_migration.puml", """
@startuml
!include <archimate/Archimate>

Implementation_WorkPackage(wp1, "Phase 1: Foundation")
Implementation_WorkPackage(wp2, "Phase 2: Migration")
Implementation_WorkPackage(wp3, "Phase 3: Cutover")
Implementation_Deliverable(del1, "Architecture Doc")
Implementation_Deliverable(del2, "Migrated Data")
Implementation_Gap(gap, "Legacy Gap")
Implementation_Plateau(p1, "Current State")
Implementation_Plateau(p2, "Target State")

Rel_Association(wp1, del1, "produces")
Rel_Association(wp2, del2, "produces")
Rel_Triggering(wp1, wp2, "")
Rel_Triggering(wp2, wp3, "")
Rel_Association(p1, gap, "has")
Rel_Association(gap, p2, "resolved by")
@enduml
""")

    # 7. Motivation view
    write("archimate", "archimate_motivation.puml", """
@startuml
!include <archimate/Archimate>

Motivation_Stakeholder(exec, "Executive")
Motivation_Stakeholder(ops, "Operations")
Motivation_Driver(d1, "Cost Reduction")
Motivation_Driver(d2, "Compliance")
Motivation_Assessment(a1, "Current Costs High")
Motivation_Goal(g1, "Reduce OPEX 20%")
Motivation_Goal(g2, "Achieve SOC2")
Motivation_Requirement(r1, "Automate Manual Processes")
Motivation_Constraint(c1, "Budget Limit $500k")
Motivation_Principle(p1, "Cloud-First")

Rel_Association(exec, d1, "concerned by")
Rel_Association(ops, d2, "concerned by")
Rel_Association(d1, a1, "")
Rel_Influence(a1, g1, "")
Rel_Realization(r1, g1, "")
Rel_Association(c1, g1, "limits")
Rel_Influence(p1, r1, "guides")
@enduml
""")

    # 8. Simple elements
    write("archimate", "archimate_simple_elements.puml", """
@startuml
!include <archimate/Archimate>

Business_Function(f1, "Finance")
Business_Function(f2, "HR")
Business_Function(f3, "IT")
Business_Process(p1, "Payroll")
Application_Component(a1, "HR System")
Application_Component(a2, "Finance System")
Technology_Node(n1, "Cloud")

Rel_Association(f2, p1, "owns")
Rel_Serving(a1, f2, "supports")
Rel_Serving(a2, f1, "supports")
Rel_Association(a1, a2, "integrates")
Rel_Composition(n1, a1, "hosts")
Rel_Composition(n1, a2, "hosts")
@enduml
""")

    # 9-50: More ArchiMate variations
    element_types = [
        "Business_Actor", "Business_Role", "Business_Process",
        "Application_Component", "Application_Service",
        "Technology_Node", "Technology_SystemSoftware"
    ]
    rel_types = [
        "Rel_Association", "Rel_Serving", "Rel_Composition",
        "Rel_Aggregation", "Rel_Triggering", "Rel_Realization"
    ]
    for i in range(9, 51):
        n = (i % 4) + 3
        elems = []
        rels = []
        for j in range(n):
            etype = element_types[(i + j) % len(element_types)]
            elems.append(f'{etype}(e{j}, "Element {j}")')
        for j in range(n-1):
            rtype = rel_types[(i + j) % len(rel_types)]
            rels.append(f'{rtype}(e{j}, e{j+1}, "")')
        content = "@startuml\n!include <archimate/Archimate>\n\n"
        content += "\n".join(elems)
        content += "\n\n"
        content += "\n".join(rels)
        content += "\n@enduml"
        write("archimate", f"archimate_variant_{i:03d}.puml", content)

gen_archimate()


# ==============================================================================
# DITAA (WIRE) DIAGRAMS
# ==============================================================================

def gen_ditaa():
    # 1. Basic boxes
    write("wire", "ditaa_basic_boxes.puml", """
@startditaa
+--------+   +--------+
|        |   |        |
| Box A  |   | Box B  |
|        |   |        |
+--------+   +--------+
@endditaa
""")

    # 2. Arrows
    write("wire", "ditaa_arrows.puml", """
@startditaa
+-------+     +-------+     +-------+
|       |     |       |     |       |
| Start |---->| Proc  |---->|  End  |
|       |     |       |     |       |
+-------+     +-------+     +-------+
@endditaa
""")

    # 3. Rounded corners
    write("wire", "ditaa_rounded.puml", r"""
@startditaa
/-------\     /-------\
|       |     |       |
| Round |---->| Round |
|       |     |       |
\-------/     \-------/
@endditaa
""")

    # 4. Colors
    write("wire", "ditaa_colors.puml", """
@startditaa
+--------+ +--------+ +--------+
|{c}     | |{r}     | |{g}     |
|  Cyan  | |  Red   | | Green  |
+--------+ +--------+ +--------+

+--------+ +--------+
|{y}     | |{b}     |
| Yellow | |  Blue  |
+--------+ +--------+
@endditaa
""")

    # 5. Document shape
    write("wire", "ditaa_document.puml", """
@startditaa
+--------+
|        |
| Report |
|        |
+---/----+
@endditaa
""")

    # 6. Storage shape
    write("wire", "ditaa_storage.puml", """
@startditaa
/--------\
|        |
| Storage|
|        |
+--------+
@endditaa
""")

    # 7. Complex layout - web architecture
    write("wire", "ditaa_web_arch.puml", """
@startditaa
               +---------------+
               |               |
    Browser    |   Load Bal    |
 +----------+  |               |
 |          |->|  +----+----+  |
 |  Client  |  |  |LB1 |LB2 |  |
 |          |  |  +----+----+  |
 +----------+  +-------+-------+
                       |
       +---------------+---------------+
       |                               |
+------+------+                 +------+------+
|             |                 |             |
|  Web App 1  |                 |  Web App 2  |
|             |                 |             |
+------+------+                 +------+------+
       |                               |
       +---------------+---------------+
                       |
               +-------+-------+
               |               |
               |    Database   |
               |    Cluster    |
               |               |
               +---------------+
@endditaa
""")

    # 8. Pipeline diagram
    write("wire", "ditaa_pipeline.puml", """
@startditaa
+--------+    +--------+    +--------+    +--------+
|        |    |        |    |        |    |        |
| Source |--->| Filter |--->|  Map   |--->|  Sink  |
|        |    |        |    |        |    |        |
+--------+    +--------+    +--------+    +--------+
                  |                |
                  v                v
             +--------+      +--------+
             |        |      |        |
             |  Dead  |      |  Side  |
             | Letter |      | Effect |
             |        |      |        |
             +--------+      +--------+
@endditaa
""")

    # 9. Dashed lines
    write("wire", "ditaa_dashed.puml", """
@startditaa
+--------+    +--------+
|        |    |        |
|  Opt A |....|  Opt B |
|        |    |        |
+--------+    +--------+
    :               :
    :               :
+--------+    +--------+
|        |    |        |
|  Opt C |    |  Opt D |
|        |    |        |
+--------+    +--------+
@endditaa
""")

    # 10. Point markers
    write("wire", "ditaa_points.puml", """
@startditaa
   *---->-------+
   |            |
   |            v
   |         +------+
   |         |      |
   *----+    | Node |
   |    |    |      |
   v    v    +------+
+----+ +----+
|    | |    |
| A  | | B  |
|    | |    |
+----+ +----+
@endditaa
""")

    # 11. Bidirectional arrows
    write("wire", "ditaa_bidir.puml", """
@startditaa
+--------+          +--------+
|        |          |        |
| Client |<-------->| Server |
|        |          |        |
+--------+          +--------+
@endditaa
""")

    # 12. Vertical flow
    write("wire", "ditaa_vertical.puml", """
@startditaa
+----------+
|          |
|  Input   |
|          |
+----+-----+
     |
     v
+----+-----+
|          |
| Process  |
|          |
+----+-----+
     |
     v
+----+-----+
|          |
|  Output  |
|          |
+----------+
@endditaa
""")

    # 13. Grid layout
    write("wire", "ditaa_grid.puml", """
@startditaa
+------+------+------+
|      |      |      |
| (1,1)| (1,2)| (1,3)|
|      |      |      |
+------+------+------+
|      |      |      |
| (2,1)| (2,2)| (2,3)|
|      |      |      |
+------+------+------+
|      |      |      |
| (3,1)| (3,2)| (3,3)|
|      |      |      |
+------+------+------+
@endditaa
""")

    # 14. Network topology
    write("wire", "ditaa_network.puml", """
@startditaa
           +--------+
           |        |
           | Router |
           |        |
           +---+----+
               |
    +----------+-----------+
    |          |           |
+---+--+   +---+--+   +---+--+
|      |   |      |   |      |
| PC 1 |   | PC 2 |   | PC 3 |
|      |   |      |   |      |
+------+   +------+   +------+
@endditaa
""")

    # 15. Mixed annotations
    write("wire", "ditaa_mixed.puml", r"""
@startditaa
 /---------\      /---------\
 | cGRE    |      | cBLU    |
 | Service |----->| Cache   |
 | Layer   |      |         |
 \---------/      \---------/
      |
      v
 /---------\
 | cRED    |
 | Data    |
 | Store   |
 \---------/
@endditaa
""")

    # 16. ASCII art table
    write("wire", "ditaa_table.puml", """
@startditaa
+----------+----------+----------+----------+
| Name     | Type     | Size     | Nullable |
+----------+----------+----------+----------+
| id       | INT      | 4        | NO       |
+----------+----------+----------+----------+
| name     | VARCHAR  | 100      | NO       |
+----------+----------+----------+----------+
| email    | VARCHAR  | 200      | YES      |
+----------+----------+----------+----------+
| created  | DATETIME | 8        | NO       |
+----------+----------+----------+----------+
@endditaa
""")

    # 17-100: Generated ditaa variants
    shapes = [
        "+------+\n|      |\n|  {}  |\n|      |\n+------+",
        "/------\\\n|      |\n|  {}  |\n|      |\n\\------/",
    ]
    for i in range(17, 101):
        n = (i % 3) + 2
        boxes = [f"Box{j}" for j in range(1, n+1)]
        arrow = "-->" if i % 2 == 0 else "--->"
        line = f"  {'  '.join(['| {:6s} |'.format(b) for b in boxes])}  "
        top = f"  {'  '.join(['+--------+' for _ in boxes])}  "
        mid1 = f"  {'  '.join(['|        |' for _ in boxes])}  "
        labels = f"  {'  '.join(['| {:6s} |'.format(b) for b in boxes])}  "
        bot = f"  {'  '.join(['+--------+' for _ in boxes])}  "
        arrow_row = "  " + ("-->".join(["        " for _ in range(n)])) + "  "

        content = "@startditaa\n"
        content += top + "\n"
        content += mid1 + "\n"
        content += labels + "\n"
        content += mid1 + "\n"
        content += bot + "\n"
        content += "@endditaa"
        write("wire", f"ditaa_variant_{i:03d}.puml", content)

gen_ditaa()


# ==============================================================================
# REGEX DIAGRAMS
# ==============================================================================

def gen_regex():
    patterns = [
        ("regex_simple_literal", "abc"),
        ("regex_alternation", "cat|dog|fish"),
        ("regex_star", "ab*c"),
        ("regex_plus", "ab+c"),
        ("regex_question", "colou?r"),
        ("regex_char_class", "[a-z]"),
        ("regex_digit_class", "[0-9]"),
        ("regex_negated_class", "[^aeiou]"),
        ("regex_backslash_d", "\\d+"),
        ("regex_backslash_w", "\\w+"),
        ("regex_backslash_s", "\\s+"),
        ("regex_dot", "a.b"),
        ("regex_anchor_start", "^Hello"),
        ("regex_anchor_end", "world$"),
        ("regex_both_anchors", "^start.*end$"),
        ("regex_quantifier_exact", "a{3}"),
        ("regex_quantifier_min", "a{2,}"),
        ("regex_quantifier_range", "a{2,5}"),
        ("regex_group", "(ab)+"),
        ("regex_non_capture", "(?:ab)+"),
        ("regex_lookahead", "foo(?=bar)"),
        ("regex_lookbehind", "(?<=foo)bar"),
        ("regex_email", "[\\w.]+@[\\w.]+\\.\\w{2,}"),
        ("regex_ipv4", "\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}"),
        ("regex_url", "https?://[\\w./%-]+"),
        ("regex_date_iso", "\\d{4}-\\d{2}-\\d{2}"),
        ("regex_time", "\\d{2}:\\d{2}(:\\d{2})?"),
        ("regex_hex_color", "#[0-9a-fA-F]{6}"),
        ("regex_phone_us", "\\(?\\d{3}\\)?[-.\\s]?\\d{3}[-.\\s]?\\d{4}"),
        ("regex_zip_code", "\\d{5}(-\\d{4})?"),
        ("regex_uuid", "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"),
        ("regex_identifier", "[a-zA-Z_][a-zA-Z0-9_]*"),
        ("regex_float", "-?\\d+\\.\\d+([eE][+-]?\\d+)?"),
        ("regex_int", "-?\\d+"),
        ("regex_whitespace_trim", "^\\s+|\\s+$"),
        ("regex_word_boundary", "\\bword\\b"),
        ("regex_nested_groups", "((a|b)(c|d))+"),
        ("regex_optional_group", "(\\+1\\s?)?\\(?\\d{3}\\)?[\\s.-]?\\d{3}[\\s.-]?\\d{4}"),
        ("regex_multiline_flag", "(?m)^line.*"),
        ("regex_case_insensitive", "(?i)hello"),
        ("regex_dotall", "(?s)start.*end"),
        ("regex_comment_mode", "(?x) \\d{4} - \\d{2} - \\d{2}"),
        ("regex_named_group", "(?P<year>\\d{4})-(?P<month>\\d{2})"),
        ("regex_backreference", "(\\w+)\\s+\\1"),
        ("regex_credit_card", "\\d{4}[\\s-]?\\d{4}[\\s-]?\\d{4}[\\s-]?\\d{4}"),
        ("regex_password", "^(?=.*[A-Z])(?=.*[0-9])(?=.*[!@#$]).{8,}$"),
        ("regex_html_tag", "<([a-z]+)[^>]*>"),
        ("regex_css_class", "\\.[a-zA-Z][a-zA-Z0-9_-]*"),
        ("regex_json_string", '"([^"\\\\]|\\\\.)*"'),
        ("regex_multiword", "\\b\\w+\\s+\\w+\\b"),
    ]

    for name, pattern in patterns:
        write("regex", f"{name}.puml", f"""
@startregex
{pattern}
@endregex
""")

gen_regex()


# ==============================================================================
# MATH / LATEX DIAGRAMS
# ==============================================================================

def gen_math():
    math_cases = [
        ("math_simple_fraction", "@startmath", "\\frac{1}{2}", "@endmath"),
        ("math_quadratic", "@startmath", "x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}", "@endmath"),
        ("math_integral", "@startmath", "\\int_0^{\\infty} e^{-x^2} dx = \\frac{\\sqrt{\\pi}}{2}", "@endmath"),
        ("math_sum", "@startmath", "\\sum_{n=1}^{\\infty} \\frac{1}{n^2} = \\frac{\\pi^2}{6}", "@endmath"),
        ("math_product", "@startmath", "\\prod_{p \\text{ prime}} \\frac{1}{1-p^{-s}}", "@endmath"),
        ("math_matrix_2x2", "@startmath", "\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}", "@endmath"),
        ("math_matrix_3x3", "@startmath", "\\begin{pmatrix} 1 & 0 & 0 \\\\ 0 & 1 & 0 \\\\ 0 & 0 & 1 \\end{pmatrix}", "@endmath"),
        ("math_determinant", "@startmath", "\\det(A) = \\begin{vmatrix} a & b \\\\ c & d \\end{vmatrix} = ad - bc", "@endmath"),
        ("math_limit", "@startmath", "\\lim_{x \\to \\infty} \\frac{1}{x} = 0", "@endmath"),
        ("math_derivative", "@startmath", "\\frac{d}{dx}\\left(x^n\\right) = nx^{n-1}", "@endmath"),
        ("math_partial", "@startmath", "\\frac{\\partial f}{\\partial x} = 2x + y", "@endmath"),
        ("math_greek_letters", "@startmath", "\\alpha + \\beta = \\gamma \\cdot \\delta", "@endmath"),
        ("math_vector_dot", "@startmath", "\\vec{a} \\cdot \\vec{b} = |\\vec{a}||\\vec{b}|\\cos(\\theta)", "@endmath"),
        ("math_vector_cross", "@startmath", "\\vec{a} \\times \\vec{b} = |\\vec{a}||\\vec{b}|\\sin(\\theta)\\hat{n}", "@endmath"),
        ("math_eulers", "@startmath", "e^{i\\pi} + 1 = 0", "@endmath"),
        ("math_fourier", "@startmath", "F(\\omega) = \\int_{-\\infty}^{\\infty} f(t) e^{-i\\omega t} dt", "@endmath"),
        ("math_taylor", "@startmath", "e^x = \\sum_{n=0}^{\\infty} \\frac{x^n}{n!}", "@endmath"),
        ("math_binomial", "@startmath", "(x+y)^n = \\sum_{k=0}^{n} \\binom{n}{k} x^k y^{n-k}", "@endmath"),
        ("math_norm", "@startmath", "\\|x\\| = \\sqrt{\\sum_{i=1}^{n} x_i^2}", "@endmath"),
        ("math_probability", "@startmath", "P(A|B) = \\frac{P(B|A) P(A)}{P(B)}", "@endmath"),
        ("math_stirling", "@startmath", "n! \\approx \\sqrt{2\\pi n} \\left(\\frac{n}{e}\\right)^n", "@endmath"),
        ("math_trig_identity", "@startmath", "\\sin^2(x) + \\cos^2(x) = 1", "@endmath"),
        ("math_complex", "@startmath", "z = a + bi, \\quad |z| = \\sqrt{a^2 + b^2}", "@endmath"),
        ("math_set_theory", "@startmath", "A \\cup B = \\{x \\mid x \\in A \\lor x \\in B\\}", "@endmath"),
        ("math_cases", "@startmath", "f(x) = \\begin{cases} x^2 & x \\geq 0 \\\\ -x^2 & x < 0 \\end{cases}", "@endmath"),
    ]

    latex_cases = [
        ("latex_equation", "@startlatex", "E = mc^2", "@endlatex"),
        ("latex_align", "@startlatex", "\\begin{align} x &= a + b \\\\ y &= c + d \\end{align}", "@endlatex"),
        ("latex_integral_def", "@startlatex", "\\int_a^b f(x)\\,dx = F(b) - F(a)", "@endlatex"),
        ("latex_schrodinger", "@startlatex", "i\\hbar\\frac{\\partial}{\\partial t}\\Psi = \\hat{H}\\Psi", "@endlatex"),
        ("latex_maxwell1", "@startlatex", "\\nabla \\cdot \\vec{E} = \\frac{\\rho}{\\epsilon_0}", "@endlatex"),
        ("latex_maxwell2", "@startlatex", "\\nabla \\times \\vec{B} = \\mu_0 \\vec{J} + \\mu_0\\epsilon_0 \\frac{\\partial \\vec{E}}{\\partial t}", "@endlatex"),
        ("latex_lorentz", "@startlatex", "F = q(\\vec{E} + \\vec{v} \\times \\vec{B})", "@endlatex"),
        ("latex_entropy", "@startlatex", "S = -k_B \\sum_i p_i \\ln p_i", "@endlatex"),
        ("latex_navier_stokes", "@startlatex", "\\rho\\frac{D\\vec{v}}{Dt} = -\\nabla p + \\mu \\nabla^2 \\vec{v} + \\vec{f}", "@endlatex"),
        ("latex_christoffel", "@startlatex", "\\Gamma^\\lambda_{\\mu\\nu} = \\frac{1}{2}g^{\\lambda\\sigma}\\left(\\partial_\\mu g_{\\nu\\sigma} + \\partial_\\nu g_{\\mu\\sigma} - \\partial_\\sigma g_{\\mu\\nu}\\right)", "@endlatex"),
        ("latex_einstein", "@startlatex", "G_{\\mu\\nu} + \\Lambda g_{\\mu\\nu} = \\frac{8\\pi G}{c^4} T_{\\mu\\nu}", "@endlatex"),
        ("latex_dirac", "@startlatex", "(i\\gamma^\\mu \\partial_\\mu - m)\\psi = 0", "@endlatex"),
        ("latex_uncertainty", "@startlatex", "\\Delta x \\cdot \\Delta p \\geq \\frac{\\hbar}{2}", "@endlatex"),
        ("latex_riemann", "@startlatex", "\\zeta(s) = \\sum_{n=1}^{\\infty} \\frac{1}{n^s}", "@endlatex"),
        ("latex_gaussian", "@startlatex", "f(x) = \\frac{1}{\\sigma\\sqrt{2\\pi}} e^{-\\frac{(x-\\mu)^2}{2\\sigma^2}}", "@endlatex"),
        ("latex_chi_square", "@startlatex", "\\chi^2 = \\sum_i \\frac{(O_i - E_i)^2}{E_i}", "@endlatex"),
        ("latex_covariance", "@startlatex", "\\text{Cov}(X,Y) = E[(X-\\mu_X)(Y-\\mu_Y)]", "@endlatex"),
        ("latex_gradient", "@startlatex", "\\nabla f = \\left(\\frac{\\partial f}{\\partial x}, \\frac{\\partial f}{\\partial y}, \\frac{\\partial f}{\\partial z}\\right)", "@endlatex"),
        ("latex_divergence", "@startlatex", "\\nabla \\cdot \\vec{F} = \\frac{\\partial F_x}{\\partial x} + \\frac{\\partial F_y}{\\partial y} + \\frac{\\partial F_z}{\\partial z}", "@endlatex"),
        ("latex_curl", "@startlatex", "\\nabla \\times \\vec{F} = \\det \\begin{pmatrix} \\hat{i} & \\hat{j} & \\hat{k} \\\\ \\partial_x & \\partial_y & \\partial_z \\\\ F_x & F_y & F_z \\end{pmatrix}", "@endlatex"),
        ("latex_stokes", "@startlatex", "\\oint_C \\vec{F} \\cdot d\\vec{r} = \\iint_S (\\nabla \\times \\vec{F}) \\cdot d\\vec{S}", "@endlatex"),
        ("latex_green", "@startlatex", "\\oint_{\\partial D} (P\\,dx + Q\\,dy) = \\iint_D \\left(\\frac{\\partial Q}{\\partial x} - \\frac{\\partial P}{\\partial y}\\right)\\,dA", "@endlatex"),
        ("latex_cauchy", "@startlatex", "f(z_0) = \\frac{1}{2\\pi i} \\oint_C \\frac{f(z)}{z - z_0}\\,dz", "@endlatex"),
        ("latex_residue", "@startlatex", "\\oint_C f(z)\\,dz = 2\\pi i \\sum_k \\text{Res}(f, z_k)", "@endlatex"),
        ("latex_poisson", "@startlatex", "P(X = k) = \\frac{\\lambda^k e^{-\\lambda}}{k!}", "@endlatex"),
    ]

    for name, start, formula, end in math_cases:
        write("math", f"{name}.puml", f"""
{start}
{formula}
{end}
""")

    for name, start, formula, end in latex_cases:
        write("math", f"{name}.puml", f"""
{start}
{formula}
{end}
""")

gen_math()


# ==============================================================================
# SPRITE TESTS
# ==============================================================================

def gen_sprites():
    # 1. Basic sprite definition
    write("sprites", "sprite_basic_def.puml", """
@startuml
sprite $cloud [15x15/16] {
000000000000000
000001110000000
000111111100000
001111111110000
011111111111000
111111111111100
111111111111110
111111111111110
111111111111110
111111111111110
111111111111100
011111111111000
001111111110000
000111111100000
000000000000000
}

[App] as app
cloud $cloud : "Cloud"

app --> cloud
@enduml
""")

    # 2. Sprite in class
    write("sprites", "sprite_in_class.puml", """
@startuml
sprite $db [16x16/16] {
0000000000000000
0011111111110000
0111111111111000
1111111111111100
1111111111111100
1111111111111100
0111111111111000
0011111111110000
0000000000000000
1111111111111100
1111111111111100
1111111111111100
0111111111111000
0011111111110000
0000000000000000
0000000000000000
}

class Database <<$db>> {
  + connect()
  + query()
  + close()
}
@enduml
""")

    # 3. Sprite in note
    write("sprites", "sprite_in_note.puml", """
@startuml
sprite $warning [11x11/16] {
00000000000
00001100000
00011110000
00110011000
01100001100
11000000110
11000000110
11111111110
11111111110
00000000000
00000000000
}

class Service {
  + process()
}

note right of Service
  <$warning> This service has known issues
end note
@enduml
""")

    # 4. Multiple sprites
    write("sprites", "sprite_multiple.puml", """
@startuml
sprite $user [8x8/16] {
00011000
00111100
00111100
00011000
01111110
11111111
11111111
01111110
}

sprite $server [8x8/16] {
11111111
11111111
10000001
10111101
10111101
10000001
11111111
11111111
}

actor "<$user> User" as user
node "<$server> Server" as server

user -> server : request
server -> user : response
@enduml
""")

    # 5-50: Sprite variation tests
    for i in range(5, 51):
        size = (i % 4 + 1) * 4
        rows = ["".join("1" if (r + c) % 3 != 0 else "0" for c in range(size)) for r in range(size)]
        sprite_body = "\n".join(rows)
        write("sprites", f"sprite_variant_{i:03d}.puml", f"""
@startuml
sprite $icon{i} [{size}x{size}/16] {{
{sprite_body}
}}

rectangle "Element <$icon{i}>" as elem
@enduml
""")

gen_sprites()


# ==============================================================================
# LINK / URL TESTS
# ==============================================================================

def gen_links():
    # 1. Class with link
    write("links", "link_class_basic.puml", """
@startuml
class MyClass [[https://example.com]] {
  + method()
}
@enduml
""")

    # 2. Class with tooltip
    write("links", "link_class_tooltip.puml", """
@startuml
class Service [[https://docs.example.com/service{API Documentation}]] {
  + process()
  + validate()
}
@enduml
""")

    # 3. Class with custom label
    write("links", "link_class_label.puml", """
@startuml
class Repository [[https://github.com/example/repo Repo Source]] {
  + save()
  + find()
}
@enduml
""")

    # 4. Note with link
    write("links", "link_note.puml", """
@startuml
class Foo {
  + bar()
}
note right of Foo
  See [[https://example.com docs]] for details
end note
@enduml
""")

    # 5. Multiple classes with links
    write("links", "link_multiple_classes.puml", """
@startuml
class Controller [[https://example.com/controller]] {
  + handle()
}
class Service [[https://example.com/service]] {
  + execute()
}
class Repository [[https://example.com/repository]] {
  + store()
}
Controller -> Service
Service -> Repository
@enduml
""")

    # 6. Sequence with links
    write("links", "link_sequence.puml", """
@startuml
actor User [[https://example.com/user]]
participant API [[https://example.com/api]]
database DB [[https://example.com/db]]

User -> API : request
API -> DB : query
DB --> API : result
API --> User : response
@enduml
""")

    # 7. Component with links
    write("links", "link_component.puml", """
@startuml
component Frontend [[https://example.com/frontend]]
component Backend [[https://example.com/backend]]
database Storage [[https://example.com/storage]]

Frontend --> Backend
Backend --> Storage
@enduml
""")

    # 8. All link variants in one diagram
    write("links", "link_all_variants.puml", """
@startuml
class A [[https://a.example.com]] {
  note: basic link
}
class B [[https://b.example.com{Tooltip for B}]] {
  note: with tooltip
}
class C [[https://c.example.com Label C]] {
  note: with label
}
class D [[https://d.example.com{Tooltip D} Label D]] {
  note: with both
}
A --> B
B --> C
C --> D
@enduml
""")

    # 9-50: More link tests per diagram type
    for i in range(9, 51):
        url = f"https://example.com/resource/{i}"
        tooltip = f"Resource {i} documentation"
        write("links", f"link_variant_{i:03d}.puml", f"""
@startuml
class Resource{i} [[{url}{{{tooltip}}}]] {{
  * id : int
  + getName() : String
  + setName(n : String) : void
}}
note right of Resource{i}
  [[{url} Click here]] for more info
end note
@enduml
""")

gen_links()


# ==============================================================================
# COMBO / KITCHEN-SINK DIAGRAMS
# ==============================================================================

def gen_combo():
    # 1. Class diagram with everything
    write("combo", "combo_class_everything.puml", """
@startuml
skinparam classBackgroundColor #F8F8FF
skinparam classBorderColor #333333
skinparam classArrowColor #666666
skinparam classFontStyle Bold
skinparam packageBackgroundColor #FFFDE7
skinparam stereotypeCBackgroundColor #E8F5E9

title "Comprehensive Class Diagram"
header Generated by RustUML
footer Page 1 of 1
caption Full class hierarchy example

package "Core" {
  abstract class Shape <<abstract>> {
    # x : double
    # y : double
    # color : Color
    + {abstract} area() : double
    + {abstract} perimeter() : double
    + move(dx : double, dy : double) : void
    + getColor() : Color
    + setColor(c : Color) : void
  }

  class Circle extends Shape {
    - radius : double
    + Circle(r : double)
    + area() : double
    + perimeter() : double
    + getRadius() : double
  }

  class Rectangle extends Shape {
    - width : double
    - height : double
    + Rectangle(w : double, h : double)
    + area() : double
    + perimeter() : double
    + getWidth() : double
    + getHeight() : double
  }

  class Square extends Rectangle {
    + Square(side : double)
  }

  interface Drawable <<interface>> {
    + draw(canvas : Canvas) : void
    + hide() : void
    + show() : void
  }

  interface Serializable <<interface>> {
    + serialize() : String
    + deserialize(s : String) : void
  }

  enum Color {
    RED
    GREEN
    BLUE
    YELLOW
    WHITE
    BLACK
  }
}

package "Canvas" {
  class Canvas {
    - shapes : List<Shape>
    - backgroundColor : Color
    + addShape(s : Shape) : void
    + removeShape(s : Shape) : boolean
    + render() : void
    + clear() : void
  }

  class Renderer {
    - canvas : Canvas
    + Renderer(c : Canvas)
    + render() : void
    + renderShape(s : Shape) : void
  }
}

Shape ..|> Drawable
Shape ..|> Serializable
Canvas "1" o-- "0..*" Shape : contains
Canvas ..> Renderer : uses
Renderer --> Canvas

note on link
  Canvas and Renderer
  are tightly coupled
end note

note top of Shape
  Base class for all
  geometric shapes
end note

note bottom of Color
  Supported color values
end note
@enduml
""")

    # 2. Sequence diagram with everything
    write("combo", "combo_sequence_everything.puml", """
@startuml
skinparam sequenceArrowThickness 2
skinparam sequenceGroupBackgroundColor #F3F3F3
skinparam sequenceLifeLineBorderColor #999999

title "Full-Featured Sequence Diagram"
header Auth Flow v2.0
footer Confidential

actor User as U
participant "Browser" as B
participant "API Gateway" as GW
participant "Auth Service" as AUTH
participant "User Service" as USR
database "DB" as DB
queue "Event Bus" as EB

autonumber

== Login Flow ==

U -> B : enter credentials
B -> GW : POST /auth/login
activate GW

GW -> AUTH : validate(credentials)
activate AUTH

AUTH -> DB : SELECT user WHERE email=?
activate DB
DB --> AUTH : user_record
deactivate DB

alt valid credentials
  AUTH -> AUTH : generate_tokens()
  AUTH --> GW : {access_token, refresh_token}
  deactivate AUTH

  GW -> EB : publish(UserLoggedIn)
  GW --> B : 200 OK + tokens
  B --> U : dashboard

else invalid credentials
  AUTH --> GW : 401 Unauthorized
  deactivate AUTH
  GW --> B : 401 Unauthorized
  B --> U : error message
end

deactivate GW

== Profile Update ==

U -> B : edit profile
B -> GW : PUT /users/me
activate GW
note right of GW : Requires valid JWT

GW -> AUTH : verify_token(jwt)
activate AUTH
AUTH --> GW : valid / {user_id}
deactivate AUTH

GW -> USR : updateProfile(user_id, data)
activate USR
USR -> DB : UPDATE users SET ...
activate DB
DB --> USR : 1 row updated
deactivate DB

USR -> EB : publish(ProfileUpdated)
USR --> GW : updated_profile
deactivate USR

GW --> B : 200 OK
B --> U : success

deactivate GW

loop every 15 min
  B -> GW : POST /auth/refresh
  GW -> AUTH : refresh(token)
  AUTH --> GW : new_access_token
  GW --> B : 200 OK + new_token
end

group Error Handling
  B -> GW : any_request
  GW -->x B : 500 Internal Error
  note right : Log and alert
end

@enduml
""")

    # 3. Activity with everything
    write("combo", "combo_activity_everything.puml", """
@startuml
skinparam activityBackgroundColor #E3F2FD
skinparam activityBorderColor #1565C0
skinparam activityArrowColor #1565C0

title "Comprehensive Activity Diagram"

start
:Initialize System;
note right: Load config from file

fork
  :Start HTTP Server;
  :Bind Port 8080;
fork again
  :Connect to Database;
  :Run Migrations;
fork again
  :Load Cache;
  :Warm Up Cache;
end fork

:System Ready;

while (request available?) is (yes)
  :Accept Connection;

  if (is authenticated?) then (yes)
    if (is authorized?) then (yes)
      switch (method)
        case (GET)
          :Fetch Resource;
          if (in cache?) then (yes)
            :Return Cached Response;
          else (no)
            :Query Database;
            :Cache Result;
            :Return Response;
          endif
        case (POST)
          :Parse Request Body;
          if (valid?) then (yes)
            :Create Resource;
            :Emit Event;
            :Return 201 Created;
          else (no)
            :Return 400 Bad Request;
          endif
        case (PUT)
          :Parse Request Body;
          :Update Resource;
          :Return 200 OK;
        case (DELETE)
          :Delete Resource;
          :Return 204 No Content;
      end switch
    else (no)
      :Return 403 Forbidden;
    endif
  else (no)
    :Return 401 Unauthorized;
  endif

  :Log Request;
  :Update Metrics;
endwhile (shutdown signal)

:Drain Active Connections;
:Flush Logs;
:Close Database;
stop
@enduml
""")

    # 4. State diagram with everything
    write("combo", "combo_state_everything.puml", """
@startuml
skinparam state {
  BackgroundColor #E8F5E9
  BorderColor #2E7D32
  ArrowColor #1B5E20
}

title "Traffic Light Controller"

[*] --> PowerOff

state PowerOff {
  [*] --> Idle
  Idle --> Initializing : power_on
  Initializing --> [*]
}

state Operating {
  [*] --> RedPhase

  state RedPhase {
    [*] --> SolidRed
    SolidRed --> FlashingRed : timer_5s / flash()
    FlashingRed --> [*] : timer_3s
  }

  state GreenPhase {
    [*] --> SolidGreen
    SolidGreen --> FlashingGreen : timer_30s / flash()
    FlashingGreen --> [*] : timer_5s
  }

  state YellowPhase {
    [*] --> SolidYellow
    SolidYellow --> [*] : timer_4s
  }

  RedPhase --> GreenPhase : [pedestrian_waiting] / green_on()
  GreenPhase --> YellowPhase : [time_elapsed] / yellow_on()
  YellowPhase --> RedPhase : [time_elapsed] / red_on()

  --

  state EmergencyOverride {
    [*] --> AllRed
    AllRed --> [*] : clear_emergency
  }
}

state Maintenance {
  [*] --> Diagnostic
  Diagnostic --> Repair : fault_detected
  Repair --> Testing : repair_done
  Testing --> [*] : test_passed
  Testing --> Repair : test_failed
}

PowerOff --> Operating : power_on / init()
Operating --> Maintenance : fault / alert()
Operating --> PowerOff : power_off / shutdown()
Maintenance --> Operating : clear / resume()

note right of Operating : Normal operation mode
note left of Maintenance : Technical staff required

[*] --> PowerOff
@enduml
""")

    # 5. Component diagram with everything
    write("combo", "combo_component_everything.puml", """
@startuml
skinparam componentStyle rectangle
skinparam component {
  BackgroundColor #EDE7F6
  BorderColor #4527A0
}
skinparam interface {
  BackgroundColor #FFFFFF
  BorderColor #4527A0
}

title "Microservice Architecture"

package "Client Tier" {
  [Web Browser] as browser
  [Mobile App] as mobile
}

package "Edge Tier" {
  [CDN] as cdn
  [Load Balancer] as lb
  interface "HTTPS" as https
}

package "API Tier" {
  [API Gateway] as gateway
  interface "REST" as rest
  interface "gRPC" as grpc
  [Rate Limiter] as rl
  [Auth Middleware] as auth_mw
}

package "Business Services" {
  [User Service] as usersvc
  [Order Service] as ordersvc
  [Product Service] as prodsvc
  [Notification Service] as notif
  [Payment Service] as payment
  interface "Events" as events
}

package "Data Tier" {
  database "User DB\n(PostgreSQL)" as userdb
  database "Order DB\n(PostgreSQL)" as orderdb
  database "Product DB\n(MongoDB)" as productdb
  database "Session Cache\n(Redis)" as cache
  queue "Event Bus\n(Kafka)" as kafka
}

package "Infrastructure" {
  [Log Aggregator] as logs
  [Metrics Collector] as metrics
  [Tracing Service] as tracing
  [Config Service] as config
}

browser --> cdn
mobile --> cdn
cdn --> lb
lb --> https
https -- gateway
gateway --> rl
rl --> auth_mw
auth_mw --> rest
auth_mw --> grpc
rest -- usersvc
rest -- ordersvc
rest -- prodsvc
grpc -- payment
usersvc --> userdb
usersvc --> cache
ordersvc --> orderdb
ordersvc --> events
prodsvc --> productdb
events -- kafka
kafka --> notif
kafka --> payment
usersvc ..> logs
ordersvc ..> logs
prodsvc ..> logs
usersvc ..> metrics
ordersvc ..> metrics
usersvc ..> tracing
gateway ..> config
@enduml
""")

    # 6. Realistic microservice architecture
    write("combo", "combo_microservice_arch.puml", """
@startuml
skinparam defaultTextAlignment center
skinparam rectangle {
  BackgroundColor #E3F2FD
  BorderColor #1565C0
}

title "E-Commerce Microservice Architecture"

rectangle "Customer Zone" {
  actor "Customer" as cust
  rectangle "Web SPA" as spa
  rectangle "Mobile App" as app
}

rectangle "API Gateway Zone" {
  rectangle "Nginx\nLoad Balancer" as nginx
  rectangle "API Gateway\n:8080" as apigw
  rectangle "Rate Limiter" as ratelimit
  rectangle "JWT Auth" as jwt
}

rectangle "Core Services" {
  rectangle "User Service\n:3001" as usersvc
  rectangle "Product Service\n:3002" as prodsvc
  rectangle "Order Service\n:3003" as ordersvc
  rectangle "Cart Service\n:3004" as cartsvc
  rectangle "Search Service\n:3005" as searchsvc
  rectangle "Review Service\n:3006" as reviewsvc
}

rectangle "Payment Zone" {
  rectangle "Payment Service\n:3007" as paysvc
  rectangle "Fraud Detector\n:3008" as fraud
  rectangle "Stripe Gateway" as stripe
  rectangle "PayPal Gateway" as paypal
}

rectangle "Fulfillment Zone" {
  rectangle "Inventory Service\n:3009" as invsvc
  rectangle "Shipping Service\n:3010" as shipsvc
  rectangle "Notification Service\n:3011" as notifsvc
  rectangle "Email Provider" as email
  rectangle "SMS Provider" as sms
}

rectangle "Data Zone" {
  database "User DB\nPostgres" as userdb
  database "Product DB\nMongo" as productdb
  database "Order DB\nPostgres" as orderdb
  database "Search Index\nElastic" as elastic
  database "Session Store\nRedis" as redis
  queue "Event Bus\nKafka" as kafka
}

cust --> spa
cust --> app
spa --> nginx
app --> nginx
nginx --> apigw
apigw --> ratelimit
ratelimit --> jwt
jwt --> usersvc
jwt --> prodsvc
jwt --> ordersvc
jwt --> cartsvc
jwt --> searchsvc
jwt --> reviewsvc
jwt --> paysvc

usersvc --> userdb
usersvc --> redis
prodsvc --> productdb
prodsvc --> elastic
ordersvc --> orderdb
ordersvc --> kafka
cartsvc --> redis
searchsvc --> elastic
paysvc --> fraud
paysvc --> stripe
paysvc --> paypal
paysvc --> kafka

kafka --> invsvc
kafka --> shipsvc
kafka --> notifsvc
notifsvc --> email
notifsvc --> sms
invsvc --> orderdb
@enduml
""")

    # 7. Realistic domain model
    write("combo", "combo_domain_model.puml", """
@startuml
skinparam class {
  BackgroundColor #FFF9C4
  BorderColor #F57F17
}
skinparam package {
  BackgroundColor #FFF3E0
}

title "Insurance Domain Model"

package "Policy Management" {
  class Policy {
    - id : UUID
    - number : String
    - status : PolicyStatus
    - effectiveDate : LocalDate
    - expiryDate : LocalDate
    - premium : Money
    + renew() : Policy
    + cancel(reason : String) : void
    + endorse(change : Endorsement) : Policy
  }

  enum PolicyStatus {
    DRAFT
    ACTIVE
    SUSPENDED
    CANCELLED
    EXPIRED
  }

  class Endorsement {
    - id : UUID
    - type : EndorsementType
    - effectiveDate : LocalDate
    - premiumChange : Money
    + apply(policy : Policy) : void
  }

  class Coverage {
    - type : CoverageType
    - limit : Money
    - deductible : Money
    - premium : Money
  }
}

package "Party Management" {
  abstract class Party {
    - id : UUID
    - name : String
    - contacts : List<Contact>
  }

  class Individual extends Party {
    - dob : LocalDate
    - taxId : String
    - gender : Gender
  }

  class Organization extends Party {
    - registrationNumber : String
    - taxId : String
    - industry : String
  }

  class Contact {
    - type : ContactType
    - value : String
    - isPrimary : boolean
  }

  class Address {
    - line1 : String
    - line2 : String
    - city : String
    - state : String
    - zip : String
    - country : String
  }
}

package "Claims" {
  class Claim {
    - id : UUID
    - number : String
    - status : ClaimStatus
    - lossDate : LocalDate
    - reportedDate : LocalDate
    - description : String
    - reservedAmount : Money
    - paidAmount : Money
    + open() : void
    + investigate() : void
    + settle(amount : Money) : void
    + deny(reason : String) : void
    + close() : void
  }

  enum ClaimStatus {
    OPEN
    UNDER_INVESTIGATION
    SETTLED
    DENIED
    CLOSED
  }

  class Payment {
    - id : UUID
    - amount : Money
    - method : PaymentMethod
    - processedAt : Instant
  }

  class Document {
    - id : UUID
    - type : DocumentType
    - url : String
    - uploadedAt : Instant
  }
}

package "Underwriting" {
  class RiskAssessment {
    - id : UUID
    - score : int
    - factors : Map<String, Double>
    - assessedAt : Instant
    + computePremium() : Money
  }
}

Policy "1" *-- "1..*" Coverage : includes
Policy "0..*" -- "1" Party : insures
Policy "0..*" -- "1" RiskAssessment : based on
Policy "1" *-- "0..*" Endorsement : has
Claim "0..*" -- "1" Policy : against
Claim "1" *-- "0..*" Payment : has
Claim "1" *-- "0..*" Document : supported by
Party "1" *-- "0..*" Contact : has
Party "1" *-- "0..*" Address : at
@enduml
""")

    # 8. Title + header + footer + caption + legend
    write("combo", "combo_all_decorations.puml", """
@startuml
title "Diagram With All Decorations"
header © 2024 Example Corp. All rights reserved.
footer Generated on %date("yyyy-MM-dd")% — Page %page% of %lastpage%
caption Figure 1: System Overview

class MainSystem {
  + core() : void
}

class SubSystem {
  + execute() : void
}

class HelperLib {
  + support() : void
}

MainSystem --> SubSystem : delegates
MainSystem --> HelperLib : uses

legend top right
  | Color | Meaning |
  |<#E3F2FD> Blue | System component |
  |<#E8F5E9> Green | Library |
  |<#FFF9C4> Yellow | External |
endlegend

note top of MainSystem
  Entry point for the system
end note

note bottom
  This is a bottom note
end note
@enduml
""")

    # 9. Every skinparam category
    write("combo", "combo_skinparam_showcase.puml", """
@startuml
skinparam backgroundColor #FAFAFA
skinparam defaultFontName "Helvetica"
skinparam defaultFontSize 12

skinparam class {
  BackgroundColor #E8F5E9
  BorderColor #2E7D32
  ArrowColor #1B5E20
  FontStyle Bold
  HeaderBackgroundColor #A5D6A7
}

skinparam interface {
  BackgroundColor #E3F2FD
  BorderColor #1565C0
}

skinparam note {
  BackgroundColor #FFF9C4
  BorderColor #F57F17
}

skinparam package {
  BackgroundColor #EDE7F6
  BorderColor #4527A0
  FontStyle Italic
}

skinparam stereotype {
  CBackgroundColor #FCE4EC
  CBorderColor #880E4F
  ABackgroundColor #E3F2FD
  ABorderColor #0D47A1
  IBackgroundColor #E8F5E9
  IBorderColor #1B5E20
  EBackgroundColor #FFF8E1
  EBorderColor #FF6F00
}

package "Geometry" {
  interface Measurable <<interface>> {
    + measure() : double
  }
  abstract class Shape <<abstract>> {
    + {abstract} area() : double
  }
  class Circle <<entity>> {
    - r : double
    + area() : double
  }
  enum Orientation <<enumeration>> {
    PORTRAIT
    LANDSCAPE
  }
}

Shape ..|> Measurable
Circle --|> Shape

note right of Shape : Abstract base class
note left of Circle : Concrete implementation

@enduml
""")

    # 10. Preprocessing macros
    write("combo", "combo_preprocessing_macros.puml", """
@startuml
!define SERVICE(name, port) rectangle "name\\n:port" as name##Svc
!define DB(name, type) database "name\\n(type)" as name##DB
!define CONNECTS(a, b, label) a --> b : label

SERVICE(User, 3001)
SERVICE(Order, 3002)
SERVICE(Product, 3003)
SERVICE(Payment, 3004)
SERVICE(Notification, 3005)

DB(User, PostgreSQL)
DB(Order, PostgreSQL)
DB(Product, MongoDB)

CONNECTS(UserSvc, UserDB, read/write)
CONNECTS(OrderSvc, OrderDB, read/write)
CONNECTS(ProductSvc, ProductDB, read/write)
CONNECTS(OrderSvc, UserSvc, validate user)
CONNECTS(OrderSvc, ProductSvc, check stock)
CONNECTS(OrderSvc, PaymentSvc, process)
CONNECTS(PaymentSvc, NotificationSvc, receipt)
@enduml
""")

    # 11. Combined macros + skinparams + creole
    write("combo", "combo_mixed_features.puml", """
@startuml
!define PRIMARY(name) class name <<Primary>>
!define SECONDARY(name) class name <<Secondary>>

skinparam class {
  BackgroundColor<<Primary>> #DCEDC8
  BackgroundColor<<Secondary>> #F8BBD0
  BorderColor<<Primary>> #558B2F
  BorderColor<<Secondary>> #AD1457
}

title "Mixed Features Demo"

PRIMARY(CoreEngine) {
  **Bold text** in javadoc
  --
  + {static} instance() : CoreEngine
  + process(//italicText// : String) : Result
}

SECONDARY(PluginA) {
  note: ""monospace""
  --
  + activate() : void
}

SECONDARY(PluginB) {
  note: <color:red>red text</color>
  --
  + activate() : void
}

note bottom of CoreEngine
  This engine supports:
  * Plugins
  * Pipelines
  * <b>Bold</b> and <i>italic</i>
  * <color:blue>Colored text</color>
  * ""Code snippets""
end note

CoreEngine "1" o-- "0..*" PluginA : manages
CoreEngine "1" o-- "0..*" PluginB : manages
@enduml
""")

    # 12. Large stress test - 30+ elements
    write("combo", "combo_stress_large.puml", """
@startuml
skinparam linetype ortho
skinparam defaultFontSize 10

package "Layer A" {
  class A1 { +op() }
  class A2 { +op() }
  class A3 { +op() }
  class A4 { +op() }
  class A5 { +op() }
  class A6 { +op() }
}

package "Layer B" {
  class B1 { +op() }
  class B2 { +op() }
  class B3 { +op() }
  class B4 { +op() }
  class B5 { +op() }
  class B6 { +op() }
}

package "Layer C" {
  class C1 { +op() }
  class C2 { +op() }
  class C3 { +op() }
  class C4 { +op() }
  class C5 { +op() }
  class C6 { +op() }
}

package "Layer D" {
  class D1 { +op() }
  class D2 { +op() }
  class D3 { +op() }
  class D4 { +op() }
  class D5 { +op() }
  class D6 { +op() }
}

package "Layer E" {
  class E1 { +op() }
  class E2 { +op() }
  class E3 { +op() }
  class E4 { +op() }
  class E5 { +op() }
}

A1 --> B1
A1 --> B2
A2 --> B1
A2 --> B3
A3 --> B2
A3 --> B4
A4 --> B3
A4 --> B5
A5 --> B4
A5 --> B6
A6 --> B5
A6 --> B6
B1 --> C1
B1 --> C2
B2 --> C1
B2 --> C3
B3 --> C2
B3 --> C4
B4 --> C3
B4 --> C5
B5 --> C4
B5 --> C6
B6 --> C5
B6 --> C6
C1 --> D1
C2 --> D2
C3 --> D3
C4 --> D4
C5 --> D5
C6 --> D6
D1 --> E1
D2 --> E2
D3 --> E3
D4 --> E4
D5 --> E5
D6 --> E1
@enduml
""")

    # 13. Wide fan-out (layout stress)
    write("combo", "combo_wide_fanout.puml", """
@startuml
class Root {
  + dispatch()
}
class Handler1 { + handle() }
class Handler2 { + handle() }
class Handler3 { + handle() }
class Handler4 { + handle() }
class Handler5 { + handle() }
class Handler6 { + handle() }
class Handler7 { + handle() }
class Handler8 { + handle() }
class Handler9 { + handle() }
class Handler10 { + handle() }
class Handler11 { + handle() }
class Handler12 { + handle() }

Root --> Handler1
Root --> Handler2
Root --> Handler3
Root --> Handler4
Root --> Handler5
Root --> Handler6
Root --> Handler7
Root --> Handler8
Root --> Handler9
Root --> Handler10
Root --> Handler11
Root --> Handler12
@enduml
""")

    # 14. Deep nesting (layout stress)
    write("combo", "combo_deep_nesting.puml", """
@startuml
class L1 { + f() }
class L2 { + f() }
class L3 { + f() }
class L4 { + f() }
class L5 { + f() }
class L6 { + f() }
class L7 { + f() }
class L8 { + f() }
class L9 { + f() }
class L10 { + f() }
class L11 { + f() }
class L12 { + f() }
class L13 { + f() }
class L14 { + f() }
class L15 { + f() }

L1 --> L2
L2 --> L3
L3 --> L4
L4 --> L5
L5 --> L6
L6 --> L7
L7 --> L8
L8 --> L9
L9 --> L10
L10 --> L11
L11 --> L12
L12 --> L13
L13 --> L14
L14 --> L15
@enduml
""")

    # 15. Cross-cutting connections
    write("combo", "combo_crosscutting.puml", """
@startuml
class N1 { +f() }
class N2 { +f() }
class N3 { +f() }
class N4 { +f() }
class N5 { +f() }
class N6 { +f() }
class N7 { +f() }
class N8 { +f() }

N1 --> N2
N2 --> N3
N3 --> N4
N4 --> N5
N5 --> N6
N6 --> N7
N7 --> N8
N1 --> N5
N2 --> N7
N3 --> N8
N4 --> N1
N6 --> N2
N8 --> N3
@enduml
""")

    # 16. Class vs Object (disambiguate)
    write("combo", "combo_class_vs_object.puml", """
@startuml
class Car {
  - make : String
  - model : String
  - year : int
  + start() : void
  + stop() : void
}

object "myCar : Car" as myCar {
  make = "Toyota"
  model = "Camry"
  year = 2023
}

object "herCar : Car" as herCar {
  make = "Honda"
  model = "Civic"
  year = 2022
}

myCar ..|> Car : instanceOf
herCar ..|> Car : instanceOf
@enduml
""")

    # 17. Component vs Deployment
    write("combo", "combo_component_vs_deployment.puml", """
@startuml
node "Production Server" as prod {
  component "Web App" as webapp {
    [Frontend Module] as fe
    [API Module] as api
    [Auth Module] as auth
  }
  database "Local Cache" as cache
}

node "Database Server" as dbserver {
  database "PostgreSQL" as pg
  database "Redis" as redis
}

node "CDN" as cdn {
  [Static Assets] as assets
}

webapp --> pg : JDBC
webapp --> redis : TCP
webapp --> cache : local
cdn --> webapp : origin pull
@enduml
""")

    # 18. Workflow with swimlanes
    write("combo", "combo_workflow_swimlanes.puml", """
@startuml
skinparam swimlaneBackgroundColor #F5F5F5
skinparam swimlaneBorderColor #9E9E9E

title "Order Processing Workflow"

|Customer|
start
:Place Order;
:Provide Payment Info;

|Payment Service|
:Validate Card;
if (Card Valid?) then (yes)
  :Authorize Payment;
  :Hold Funds;
else (no)
  :Notify Failure;
  |Customer|
  :Show Error;
  stop
endif

|Order Service|
:Create Order Record;
:Assign Order Number;
:Send Confirmation Email;

|Inventory Service|
:Check Stock;
if (In Stock?) then (yes)
  :Reserve Items;
  :Update Inventory;
else (no)
  :Backorder Items;
  :Notify Delay;
endif

|Fulfillment Service|
:Pick Items;
:Pack Order;
:Print Label;
:Hand to Carrier;

|Shipping Carrier|
:Scan Package;
:Route Package;
:Deliver Package;

|Customer|
:Receive Package;
:Confirm Delivery;
stop
@enduml
""")

    # 19. Embedded system state machine
    write("combo", "combo_embedded_state_machine.puml", """
@startuml
title "Embedded System State Machine"

[*] --> Reset

state Reset {
  [*] --> BootROM
  BootROM --> LoadBootloader : ROM OK
  LoadBootloader --> VerifyImage : loaded
  VerifyImage --> [*] : verified
  VerifyImage --> ErrorState : invalid signature
}

state Operational {
  [*] --> Idle

  state Idle {
    [*] --> LowPower
    LowPower --> Active : interrupt
    Active --> LowPower : timeout / enter_sleep()
  }

  state Processing {
    [*] --> ReadSensors
    ReadSensors --> Compute : data_ready / buffer()
    Compute --> WriteOutput : done / update()
    WriteOutput --> [*] : ack
  }

  state Communication {
    [*] --> Listen
    Listen --> Receive : data_in
    Receive --> Process : frame_complete
    Process --> Transmit : response_ready
    Transmit --> Listen : sent
  }

  Idle --> Processing : task_scheduled [CPU >= 10%]
  Processing --> Idle : task_done
  Idle --> Communication : packet_received
  Communication --> Idle : comm_done

  state ErrorRecovery {
    [*] --> LogError
    LogError --> AttemptRetry : retriable
    LogError --> FatalError : non_retriable
    AttemptRetry --> [*] : success
    AttemptRetry --> FatalError : max_retries / alert()
  }

  Processing --> ErrorRecovery : exception
  Communication --> ErrorRecovery : timeout / log()
  ErrorRecovery --> Idle : recovered
}

state ErrorState {
  [*] --> DisplayError
  DisplayError --> WaitForReset : user_ack
}

Reset --> Operational : boot_ok / start()
Operational --> ErrorState : fatal_error
Operational --> Reset : watchdog_timeout
ErrorState --> Reset : reset_requested
@enduml
""")

    # 20. Mixed creole and formatting
    write("combo", "combo_creole_rich.puml", """
@startuml
title "Creole Formatting Showcase"

class RichFormattingDemo {
  + __underline__Field : String
  + --strikethrough--Field : int
  + **boldField** : boolean
  + //italicField// : double
  + ""monospaceField"" : byte[]
}

note top of RichFormattingDemo
  = Heading 1
  == Heading 2
  === Heading 3
  ----
  * Bullet one
  * Bullet two
  ** Sub-bullet
  *** Deep sub-bullet
  # Numbered one
  # Numbered two
  ## Sub-numbered
  ----
  | Col 1 | Col 2 | Col 3 |
  | a     | b     | c     |
  | x     | y     | z     |
  ----
  <b>Bold</b> <i>Italic</i> <u>Underline</u>
  <color:red>Red</color> <color:#00FF00>Green</color>
  <back:yellow>Yellow background</back>
  <size:20>Large</size> <size:8>Small</size>
  <s>Strikethrough</s> <w>Wave</w>
  ~~strikethrough2~~
end note
@enduml
""")

    # 21-50: Additional combo variants
    diagram_types = [
        ("class", "@startuml\nclass A{+m()}\nclass B{+m()}\nA-->B\n@enduml"),
        ("sequence", "@startuml\nA->B:msg\nB-->A:reply\n@enduml"),
        ("activity", "@startuml\nstart\n:do;\nstop\n@enduml"),
        ("component", "@startuml\n[A]-->[B]\n@enduml"),
        ("state", "@startuml\n[*]-->S1\nS1-->[*]\n@enduml"),
    ]
    for i in range(21, 51):
        dtype, template = diagram_types[i % len(diagram_types)]
        n = (i % 5) + 3
        if dtype == "class":
            classes = "\n".join(f"class Class{j}_{i} {{ +method{j}() }}" for j in range(1, n+1))
            rels = "\n".join(f"Class{j}_{i} --> Class{j+1}_{i}" for j in range(1, n))
            content = f"@startuml\ntitle \"Combo Variant {i}\"\n{classes}\n{rels}\n@enduml"
        elif dtype == "sequence":
            participants = "\n".join(f"participant P{j}" for j in range(1, n+1))
            msgs = "\n".join(f"P{j} -> P{j+1} : message_{j}" for j in range(1, n))
            content = f"@startuml\ntitle \"Combo Variant {i}\"\n{participants}\n{msgs}\n@enduml"
        elif dtype == "activity":
            steps = "\n".join(f":Step {j}_{i};" for j in range(1, n+1))
            content = f"@startuml\ntitle \"Combo Variant {i}\"\nstart\n{steps}\nstop\n@enduml"
        elif dtype == "component":
            comps = "\n".join(f"[Component{j}_{i}]" for j in range(1, n+1))
            rels = "\n".join(f"[Component{j}_{i}] --> [Component{j+1}_{i}]" for j in range(1, n))
            content = f"@startuml\ntitle \"Combo Variant {i}\"\n{comps}\n{rels}\n@enduml"
        else:  # state
            states = "\n".join(f"state S{j}_{i}" for j in range(1, n+1))
            trans = f"[*] --> S1_{i}\n" + "\n".join(f"S{j}_{i} --> S{j+1}_{i}" for j in range(1, n)) + f"\nS{n}_{i} --> [*]"
            content = f"@startuml\ntitle \"Combo Variant {i}\"\n{states}\n{trans}\n@enduml"
        write("combo", f"combo_variant_{i:03d}.puml", content)

    # 51-100: Realistic workflow diagrams
    for i in range(51, 101):
        steps = (i % 8) + 4
        activities = "\n".join(f":Step {j};" for j in range(1, steps+1))
        write("combo", f"combo_workflow_{i:03d}.puml", f"""
@startuml
title "Workflow {i}"
skinparam activityBackgroundColor #E3F2FD

start
{activities}
stop
@enduml
""")

    # 101-150: Sequence variants
    for i in range(101, 151):
        n = (i % 4) + 3
        participants = "\n".join(f"participant Svc{j}" for j in range(1, n+1))
        msgs = "\n".join(f"Svc{j} -> Svc{j+1} : call_{j}_{i}" for j in range(1, n))
        write("combo", f"combo_sequence_{i:03d}.puml", f"""
@startuml
title "Sequence {i}"
{participants}
{msgs}
@enduml
""")

    # 151-200: State machine variants
    for i in range(151, 201):
        n = (i % 5) + 3
        states = "\n".join(f"state State{j}_{i}" for j in range(1, n+1))
        trans = f"[*] --> State1_{i}\n" + "\n".join(f"State{j}_{i} --> State{j+1}_{i} : event_{j}" for j in range(1, n)) + f"\nState{n}_{i} --> [*] : done"
        write("combo", f"combo_state_{i:03d}.puml", f"""
@startuml
title "State Machine {i}"
{states}
{trans}
@enduml
""")

    # 201-250: Component/deployment variants
    for i in range(201, 251):
        n = (i % 4) + 3
        comps = "\n".join(f"  [Service{j}_{i}]" for j in range(1, n+1))
        rels = "\n".join(f"[Service{j}_{i}] --> [Service{j+1}_{i}]" for j in range(1, n))
        write("combo", f"combo_component_{i:03d}.puml", f"""
@startuml
title "Component Diagram {i}"
package "System{i}" {{
{comps}
}}
{rels}
@enduml
""")

    # 251-300: ER/class hybrid
    for i in range(251, 301):
        n = (i % 4) + 3
        write("combo", f"combo_class_{i:03d}.puml", f"""
@startuml
title "Class Diagram {i}"
{chr(10).join(f'''class Entity{j}_{i} {{
  - id : Long
  + getId() : Long
  + process_{j}() : void
}}''' for j in range(1, n+1))}
{chr(10).join(f"Entity{j}_{i} --|> Entity{j+1}_{i}" for j in range(1, n))}
@enduml
""")

gen_combo()


# ==============================================================================
# EXTRA ER DIAGRAMS (to reach 800+)
# ==============================================================================

def gen_extra_er():
    """Additional ER diagrams with various relationship notations."""
    extra = [
        ("er_audit_log", """
@startuml
entity AuditLog {
  * id : BIGINT <<PK>>
  --
  table_name : VARCHAR(100)
  record_id : BIGINT
  operation : ENUM('INSERT','UPDATE','DELETE')
  old_values : JSONB
  new_values : JSONB
  changed_by : INT <<FK>>
  changed_at : TIMESTAMP
  ip_address : INET
}
entity User {
  * id : INT <<PK>>
  --
  username : VARCHAR(50)
  email : VARCHAR(200)
}
User ||--o{ AuditLog : "makes"
@enduml
"""),
        ("er_config_system", """
@startuml
entity ConfigNamespace {
  * id : INT <<PK>>
  --
  name : VARCHAR(100) UNIQUE
  description : TEXT
  owner : VARCHAR(100)
}
entity ConfigKey {
  * id : INT <<PK>>
  * namespace_id : INT <<FK>>
  --
  key : VARCHAR(200)
  value_type : ENUM
  description : TEXT
  is_required : BOOLEAN
  default_value : TEXT
}
entity ConfigValue {
  * id : INT <<PK>>
  * key_id : INT <<FK>>
  * environment : VARCHAR(50)
  --
  value : TEXT
  updated_at : TIMESTAMP
  updated_by : INT <<FK>>
}
ConfigNamespace ||--o{ ConfigKey : "contains"
ConfigKey ||--o{ ConfigValue : "has"
@enduml
"""),
        ("er_permissions", """
@startuml
entity User {
  * id : INT <<PK>>
  --
  username : VARCHAR(50)
  is_active : BOOLEAN
}
entity Role {
  * id : INT <<PK>>
  --
  name : VARCHAR(100) UNIQUE
  description : TEXT
}
entity Permission {
  * id : INT <<PK>>
  --
  resource : VARCHAR(100)
  action : VARCHAR(50)
  description : TEXT
}
entity UserRole {
  * user_id : INT <<FK>>
  * role_id : INT <<FK>>
  --
  granted_at : DATETIME
  granted_by : INT <<FK>>
}
entity RolePermission {
  * role_id : INT <<FK>>
  * permission_id : INT <<FK>>
}
User ||--o{ UserRole : "has"
Role ||--o{ UserRole : "assigned to"
Role ||--o{ RolePermission : "grants"
Permission ||--o{ RolePermission : "granted by"
@enduml
"""),
        ("er_event_sourcing", """
@startuml
entity AggregateRoot {
  * id : UUID <<PK>>
  --
  type : VARCHAR(100)
  version : INT
  created_at : TIMESTAMP
  last_updated : TIMESTAMP
}
entity DomainEvent {
  * id : UUID <<PK>>
  * aggregate_id : UUID <<FK>>
  --
  event_type : VARCHAR(100)
  event_data : JSONB
  metadata : JSONB
  sequence_number : INT
  occurred_at : TIMESTAMP
  recorded_at : TIMESTAMP
}
entity Snapshot {
  * id : UUID <<PK>>
  * aggregate_id : UUID <<FK>>
  --
  state : JSONB
  version : INT
  taken_at : TIMESTAMP
}
entity EventStream {
  * aggregate_id : UUID <<FK>>
  --
  current_version : INT
  last_event_at : TIMESTAMP
}
AggregateRoot ||--o{ DomainEvent : "emits"
AggregateRoot ||--o{ Snapshot : "snapshotted as"
AggregateRoot ||--|| EventStream : "tracked by"
@enduml
"""),
        ("er_cache_invalidation", """
@startuml
entity CacheEntry {
  * id : UUID <<PK>>
  --
  key : VARCHAR(500) UNIQUE
  value : BYTEA
  ttl_seconds : INT
  created_at : TIMESTAMP
  accessed_at : TIMESTAMP
  expires_at : TIMESTAMP
  hit_count : BIGINT
}
entity CacheTag {
  * id : INT <<PK>>
  --
  tag : VARCHAR(200) UNIQUE
}
entity CacheEntryTag {
  * entry_id : UUID <<FK>>
  * tag_id : INT <<FK>>
}
entity InvalidationRule {
  * id : INT <<PK>>
  --
  pattern : VARCHAR(500)
  event_type : VARCHAR(100)
  tag : VARCHAR(200)
  created_at : TIMESTAMP
}
CacheEntry ||--o{ CacheEntryTag : "tagged with"
CacheTag ||--o{ CacheEntryTag : "applied to"
@enduml
"""),
    ]

    for name, content in extra:
        write("er", f"{name}.puml", content)

    # Generate additional schema variants
    for i in range(151, 201):
        n = (i % 5) + 3
        write("er", f"er_extended_{i:03d}.puml", f"""
@startuml
entity Primary{i} {{
  * id : INT <<PK>>
  --
  code : VARCHAR(20)
  label : VARCHAR(100)
  is_active : BOOLEAN
  created_at : TIMESTAMP
}}
{chr(10).join(f'''entity Related{i}_{j} {{
  * id : INT <<PK>>
  * primary_id : INT <<FK>>
  --
  value : VARCHAR(200)
  sort_order : INT
}}''' for j in range(1, n+1))}
{chr(10).join(f"Primary{i} ||--o{{ Related{i}_{j} : \"has\"" for j in range(1, n+1))}
@enduml
""")

gen_extra_er()


# ==============================================================================
# EXTRA COMBO DIAGRAMS (to reach 800+)
# ==============================================================================

def gen_extra_combo():
    """Additional combo diagrams."""
    # Package-heavy class diagrams
    for i in range(301, 341):
        n = (i % 3) + 2
        packages = [f"pkg{j}" for j in range(1, n+1)]
        pkg_content = ""
        for pkg in packages:
            pkg_content += f'package "{pkg.upper()}" {{\n'
            pkg_content += f"  class {pkg}Model {{ +id : int }}\n"
            pkg_content += f"  class {pkg}Service {{ +process() }}\n"
            pkg_content += "}\n"
        rels = "\n".join(f"pkg{j}Service --> pkg{j+1}Model" for j in range(1, n))
        write("combo", f"combo_packages_{i:03d}.puml", f"""
@startuml
title "Package Diagram {i}"
{pkg_content}
{rels}
@enduml
""")

    # Deployment diagrams
    for i in range(341, 381):
        n = (i % 3) + 2
        nodes = "\n".join(f'node "Server{j}_{i}" as s{j} {{\n  component "App{j}" as a{j}\n}}' for j in range(1, n+1))
        rels = "\n".join(f"a{j} --> a{j+1}" for j in range(1, n))
        write("combo", f"combo_deployment_{i:03d}.puml", f"""
@startuml
title "Deployment Diagram {i}"
{nodes}
{rels}
@enduml
""")

gen_extra_combo()


# ==============================================================================
# Count and report
# ==============================================================================

total = 0
for subdir in DIRS:
    dirpath = os.path.join(BASE, subdir)
    if os.path.isdir(dirpath):
        count = len([f for f in os.listdir(dirpath) if f.endswith(".puml")])
        print(f"  {subdir:20s}: {count:4d} files")
        total += count

print(f"\n  {'TOTAL':20s}: {total:4d} files")
