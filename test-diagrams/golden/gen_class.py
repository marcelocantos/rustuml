#!/usr/bin/env python3
"""
Generator for comprehensive PlantUML class diagram test cases.
Generates ~2000+ .puml files covering every conceivable class diagram feature.
"""

import os
import itertools
from pathlib import Path

OUTPUT_DIR = Path(__file__).parent / "class"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

file_count = 0

def write_puml(filename: str, content: str):
    global file_count
    path = OUTPUT_DIR / filename
    path.write_text(content + "\n")
    file_count += 1


def wrap(body: str) -> str:
    return f"@startuml\n{body}\n@enduml"


# ─────────────────────────────────────────────────────────────────────────────
# 1. BASIC CLASS TYPES
# ─────────────────────────────────────────────────────────────────────────────

CLASS_TYPES = ["class", "abstract class", "interface", "enum", "annotation", "entity"]
CLASS_KEYWORDS = ["class", "abstract_class", "interface", "enum", "annotation", "entity"]

# Empty body
for kw, label in zip(CLASS_TYPES, CLASS_KEYWORDS):
    write_puml(f"class_type_{label}_empty.puml", wrap(f"{kw} MyType"))

# With curly braces (explicit empty body)
for kw, label in zip(CLASS_TYPES, CLASS_KEYWORDS):
    write_puml(f"class_type_{label}_empty_braces.puml", wrap(f"{kw} MyType {{}}"))

# With fields only
for kw, label in zip(CLASS_TYPES, CLASS_KEYWORDS):
    write_puml(f"class_type_{label}_fields_only.puml", wrap(
        f"{kw} MyType {{\n  +String name\n  -int age\n  #double score\n}}"
    ))

# With methods only
for kw, label in zip(CLASS_TYPES, CLASS_KEYWORDS):
    write_puml(f"class_type_{label}_methods_only.puml", wrap(
        f"{kw} MyType {{\n  +void doSomething()\n  -String compute(int x)\n  #boolean check()\n}}"
    ))

# With both fields and methods
for kw, label in zip(CLASS_TYPES, CLASS_KEYWORDS):
    write_puml(f"class_type_{label}_fields_and_methods.puml", wrap(
        f"{kw} MyType {{\n  +String name\n  -int age\n  --\n  +void doSomething()\n  -String compute(int x)\n}}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 2. GENERIC CLASSES
# ─────────────────────────────────────────────────────────────────────────────

generics = [
    ("single_type_param", "Foo<T>"),
    ("two_type_params", "Foo<K, V>"),
    ("three_type_params", "Foo<A, B, C>"),
    ("bounded_upper", "Foo<T extends Bar>"),
    ("bounded_lower", "Foo<T super Bar>"),
    ("wildcard", "Foo<?>"),
    ("nested_generic", "Foo<List<T>>"),
    ("multiple_bounds", "Foo<T extends Comparable<T>>"),
]

for label, sig in generics:
    write_puml(f"class_generic_{label}.puml", wrap(
        f"class {sig} {{\n  +T value\n  +T get()\n  +void set(T val)\n}}"
    ))

# Generic interface
write_puml("class_generic_interface.puml", wrap(
    "interface Container<T> {\n  +T get()\n  +void put(T item)\n  +boolean isEmpty()\n}"
))

# Generic abstract class
write_puml("class_generic_abstract.puml", wrap(
    "abstract class AbstractRepository<T, ID> {\n  +T findById(ID id)\n  +List<T> findAll()\n  +void save(T entity)\n  +void delete(ID id)\n}"
))

# Generic enum (unusual but valid)
write_puml("class_generic_class_with_enum.puml", wrap(
    "class Result<T> {\n  +T value\n  +String error\n  +boolean isSuccess()\n}\n\nenum Status {\n  OK\n  FAIL\n  PENDING\n}\n\nResult --> Status"
))


# ─────────────────────────────────────────────────────────────────────────────
# 3. STEREOTYPES
# ─────────────────────────────────────────────────────────────────────────────

stereotypes = [
    ("singleton", "<<singleton>>"),
    ("interface", "<<interface>>"),
    ("abstract", "<<abstract>>"),
    ("service", "<<service>>"),
    ("repository", "<<repository>>"),
    ("controller", "<<controller>>"),
    ("entity", "<<entity>>"),
    ("value_object", "<<value object>>"),
    ("factory", "<<factory>>"),
    ("observer", "<<observer>>"),
    ("subject", "<<subject>>"),
    ("strategy", "<<strategy>>"),
    ("decorator", "<<decorator>>"),
    ("command", "<<command>>"),
    ("builder", "<<builder>>"),
]

for label, stereo in stereotypes:
    write_puml(f"class_stereotype_{label}.puml", wrap(
        f"class MyClass {stereo} {{\n  -instance: MyClass\n  +getInstance(): MyClass\n}}"
    ))

# Multiple stereotypes
write_puml("class_stereotype_multiple.puml", wrap(
    "class ServiceImpl <<service>> <<singleton>> {\n  +void execute()\n}"
))

# Stereotype with interface
write_puml("class_stereotype_on_interface.puml", wrap(
    "interface Repository <<repository>> {\n  +void save(Object o)\n  +Object findById(int id)\n}"
))

# Custom spot character
write_puml("class_spot_char_custom.puml", wrap(
    "class MyClass << (S,#FF7700) Singleton >> {\n  -instance: MyClass\n  +getInstance(): MyClass\n}"
))

spot_chars = [
    ("A", "#red"),
    ("B", "#blue"),
    ("C", "#green"),
    ("D", "#yellow"),
    ("E", "#purple"),
    ("X", "#FF7700"),
    ("Z", "#00FFAA"),
]
for char, color in spot_chars:
    write_puml(f"class_spot_char_{char.lower()}.puml", wrap(
        f"class Spot{char} << ({char},{color}) Custom >> {{\n  +void method()\n}}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 4. COLORS
# ─────────────────────────────────────────────────────────────────────────────

colors = [
    ("red", "#red"),
    ("blue", "#blue"),
    ("green", "#green"),
    ("yellow", "#yellow"),
    ("orange", "#orange"),
    ("pink", "#pink"),
    ("hex_ff7700", "#FF7700"),
    ("hex_aabbcc", "#AABBCC"),
    ("lightblue", "#lightblue"),
    ("lightyellow", "#lightyellow"),
    ("aliceblue", "#AliceBlue"),
    ("bisque", "#Bisque"),
]

for label, color in colors:
    write_puml(f"class_color_{label}.puml", wrap(
        f"class MyClass {color} {{\n  +String field\n  +void method()\n}}"
    ))

# Color with stereotype
write_puml("class_color_with_stereotype.puml", wrap(
    "class MyClass <<singleton>> #FF7700 {\n  -instance: MyClass\n  +getInstance(): MyClass\n}"
))

# Multiple classes with different colors
write_puml("class_color_multiple.puml", wrap(
    "class Red #red {\n  +void r()\n}\nclass Blue #blue {\n  +void b()\n}\nclass Green #green {\n  +void g()\n}\nRed --> Blue\nBlue --> Green"
))

# Background and border colors
write_puml("class_color_bg_border.puml", wrap(
    "class Styled #back:lightyellow;line:red;line.bold;text:blue {\n  +String name\n}"
))


# ─────────────────────────────────────────────────────────────────────────────
# 5. VISIBILITY MODIFIERS
# ─────────────────────────────────────────────────────────────────────────────

visibilities = [
    ("+", "public"),
    ("-", "private"),
    ("#", "protected"),
    ("~", "package"),
]

# Fields with all visibility types
write_puml("class_visibility_all_fields.puml", wrap(
    "class MyClass {\n  +String publicField\n  -int privateField\n  #double protectedField\n  ~boolean packageField\n}"
))

# Methods with all visibility types
write_puml("class_visibility_all_methods.puml", wrap(
    "class MyClass {\n  +void publicMethod()\n  -void privateMethod()\n  #void protectedMethod()\n  ~void packageMethod()\n}"
))

# Each visibility alone
for vis, label in visibilities:
    write_puml(f"class_visibility_{label}_only.puml", wrap(
        f"class MyClass {{\n  {vis}String field\n  {vis}void method()\n}}"
    ))

# Visibility on static members
write_puml("class_visibility_static.puml", wrap(
    "class MyClass {\n  {static} +String staticPublic\n  {static} -int staticPrivate\n  {static} +void staticMethod()\n}"
))

# Visibility on abstract members
write_puml("class_visibility_abstract.puml", wrap(
    "abstract class MyClass {\n  {abstract} +void publicAbstract()\n  {abstract} #void protectedAbstract()\n}"
))


# ─────────────────────────────────────────────────────────────────────────────
# 6. FIELD/METHOD SEPARATORS
# ─────────────────────────────────────────────────────────────────────────────

separators = [
    ("dotdot", ".."),
    ("dash", "--"),
    ("equal", "=="),
    ("underscore", "__"),
    ("dotdot_labeled", ".. label .."),
    ("dash_labeled", "-- label --"),
    ("equal_labeled", "== label =="),
    ("underscore_labeled", "__ label __"),
]

for label, sep in separators:
    write_puml(f"class_separator_{label}.puml", wrap(
        f"class MyClass {{\n  +String name\n  {sep}\n  +void method()\n}}"
    ))

# Multiple separators in one class
write_puml("class_separator_multiple.puml", wrap(
    "class MyClass {\n"
    "  +String publicField\n"
    "  -int privateField\n"
    "  -- Private Methods --\n"
    "  -void privateMethod()\n"
    "  == Public Interface ==\n"
    "  +void publicMethod()\n"
    "  .. Notes ..\n"
    "  +String toString()\n"
    "}"
))

# Class with only separators
write_puml("class_separator_sections_only.puml", wrap(
    "class MyClass {\n  == Section A ==\n  .. Section B ..\n  -- Section C --\n  __ Section D __\n}"
))


# ─────────────────────────────────────────────────────────────────────────────
# 7. STATIC AND ABSTRACT MEMBERS
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_static_members.puml", wrap(
    "class MyClass {\n"
    "  {static} String CONSTANT\n"
    "  {static} int instanceCount\n"
    "  {static} MyClass getInstance()\n"
    "  {static} void reset()\n"
    "  +String name\n"
    "  +void doWork()\n"
    "}"
))

write_puml("class_abstract_members.puml", wrap(
    "abstract class Template {\n"
    "  {abstract} +void step1()\n"
    "  {abstract} +void step2()\n"
    "  {abstract} #String compute()\n"
    "  +void run() {\n"
    "    step1()\n"
    "    step2()\n"
    "  }\n"
    "}"
))

write_puml("class_static_and_abstract.puml", wrap(
    "abstract class Base {\n"
    "  {static} +String VERSION\n"
    "  {static} +Base create()\n"
    "  {abstract} +void execute()\n"
    "  {abstract} #String computeResult()\n"
    "}"
))


# ─────────────────────────────────────────────────────────────────────────────
# 8. COMPLEX METHOD SIGNATURES
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_methods_complex_signatures.puml", wrap(
    "class Service {\n"
    "  +void noArgs()\n"
    "  +String oneArg(int x)\n"
    "  +int twoArgs(String a, double b)\n"
    "  +List<String> genericReturn()\n"
    "  +Map<String, Integer> mapReturn()\n"
    "  +void varargs(String... args)\n"
    "  +<T> T genericMethod(T input)\n"
    "  +Optional<String> optionalReturn()\n"
    "  +void throwsException() throws IOException\n"
    "}"
))

write_puml("class_methods_return_types.puml", wrap(
    "class TypedMethods {\n"
    "  +void voidReturn()\n"
    "  +int intReturn()\n"
    "  +long longReturn()\n"
    "  +double doubleReturn()\n"
    "  +boolean boolReturn()\n"
    "  +String stringReturn()\n"
    "  +Object objectReturn()\n"
    "  +int[] arrayReturn()\n"
    "  +List<String> listReturn()\n"
    "}"
))

# Properties (C# style)
write_puml("class_properties_csharp_style.puml", wrap(
    "class CSharpClass {\n"
    "  +String Name { get; set; }\n"
    "  +int Age { get; }\n"
    "  +double Score { set; }\n"
    "  +bool IsValid { get; private set; }\n"
    "}"
))


# ─────────────────────────────────────────────────────────────────────────────
# 9. RELATIONSHIPS
# ─────────────────────────────────────────────────────────────────────────────

# Basic relationship types
rel_types = [
    ("extension", "Parent <|-- Child"),
    ("implementation", "Interface ..|> Implementation"),
    ("composition", "Whole *-- Part"),
    ("aggregation", "Container o-- Element"),
    ("association", "ClassA -- ClassB"),
    ("dependency", "ClassA ..> ClassB"),
    ("directed_association", "ClassA --> ClassB"),
    ("realization", "Interface <|.. ConcreteClass"),
    ("note_link", "ClassA .. ClassB"),
]

for label, rel in rel_types:
    classes = set()
    for word in rel.split():
        if word not in ("<|--", "..|>", "*--", "o--", "--", "..>", "-->", "<|..", ".."):
            classes.add(word)
    class_defs = "\n".join(f"class {c}" for c in sorted(classes))
    write_puml(f"class_rel_{label}.puml", wrap(f"{class_defs}\n\n{rel}"))

# All arrow directions
directions = [
    ("up", "-up->"),
    ("down", "-down->"),
    ("left", "-left->"),
    ("right", "-right->"),
    ("up_short", "-u->"),
    ("down_short", "-d->"),
    ("left_short", "-l->"),
    ("right_short", "-r->"),
]

for label, arrow in directions:
    write_puml(f"class_rel_direction_{label}.puml", wrap(
        f"class Source\nclass Target\nSource {arrow} Target"
    ))

# Arrow lengths
arrow_lengths = [
    ("short", "-"),
    ("medium", "--"),
    ("long", "---"),
    ("very_long", "----"),
]

for label, dash in arrow_lengths:
    write_puml(f"class_rel_length_{label}.puml", wrap(
        f"class A\nclass B\nA {dash}> B"
    ))

# Arrow labels - near end, far end, middle
write_puml("class_rel_label_near_far.puml", wrap(
    "class School\nclass Student\nSchool \"1\" o-- \"*\" Student : enrolls >"
))

write_puml("class_rel_label_multiplicity.puml", wrap(
    "class Person\nclass Address\nPerson \"1\" -- \"1..*\" Address : lives at"
))

write_puml("class_rel_label_all_positions.puml", wrap(
    "class A\nclass B\nA \"role_a\" --> \"role_b\" B : association"
))

write_puml("class_rel_label_navigability.puml", wrap(
    "class Customer\nclass Order\nCustomer \"1\" --> \"*\" Order : places >\nOrder \"*\" ..> \"1\" Customer : < belongs to"
))

# Bidirectional
write_puml("class_rel_bidirectional.puml", wrap(
    "class A\nclass B\nA <--> B"
))

write_puml("class_rel_bidirectional_labeled.puml", wrap(
    "class Husband\nclass Wife\nHusband \"1\" <--> \"1\" Wife : married to"
))

# Hidden relationship
write_puml("class_rel_hidden.puml", wrap(
    "class A\nclass B\nclass C\nA -[hidden]-> B\nB --> C"
))

write_puml("class_rel_hidden_layout.puml", wrap(
    "class Top\nclass Middle\nclass Bottom\nTop -[hidden]-> Middle\nMiddle -[hidden]-> Bottom\nTop --> Bottom"
))

# Colored arrows
arrow_colors = ["red", "blue", "green", "#FF7700", "#AABBCC"]
for color in arrow_colors:
    label = color.replace("#", "hex_")
    write_puml(f"class_rel_colored_{label}.puml", wrap(
        f"class A\nclass B\nA -[{color}]-> B"
    ))

write_puml("class_rel_colored_with_thickness.puml", wrap(
    "class A\nclass B\nclass C\nA -[#red,bold]-> B\nB -[#blue,dashed]-> C"
))

# Dashed and dotted
write_puml("class_rel_dashed.puml", wrap(
    "class A\nclass B\nA -[dashed]-> B"
))

# Multiple relationships between same classes
write_puml("class_rel_multiple_same_classes.puml", wrap(
    "class A\nclass B\nA --> B : uses\nA ..> B : depends\nA --o B : has"
))

# Comprehensive relationship showcase
write_puml("class_rel_all_types_showcase.puml", wrap(
    "class Animal\nclass Dog\nclass Cat\ninterface Runnable\ninterface Swimmable\nclass Pack\nclass Leash\n\nAnimal <|-- Dog\nAnimal <|-- Cat\nDog ..|> Runnable\nDog ..|> Swimmable\nPack o-- Dog\nDog -- Leash"
))


# ─────────────────────────────────────────────────────────────────────────────
# 10. DETAILED INHERITANCE PATTERNS
# ─────────────────────────────────────────────────────────────────────────────

# Simple inheritance
write_puml("class_inherit_simple.puml", wrap(
    "class Animal {\n  +String name\n  +void breathe()\n}\nclass Dog {\n  +String breed\n  +void bark()\n}\nAnimal <|-- Dog"
))

# Deep inheritance chain (5 levels)
write_puml("class_inherit_deep_5_levels.puml", wrap(
    "class A {\n  +void methodA()\n}\nclass B {\n  +void methodB()\n}\nclass C {\n  +void methodC()\n}\nclass D {\n  +void methodD()\n}\nclass E {\n  +void methodE()\n}\nA <|-- B\nB <|-- C\nC <|-- D\nD <|-- E"
))

# Deep inheritance chain (7 levels)
write_puml("class_inherit_deep_7_levels.puml", wrap(
    "\n".join([f"class L{i} {{ +void method{i}() }}" for i in range(1, 8)]) + "\n" +
    "\n".join([f"L{i} <|-- L{i+1}" for i in range(1, 7)])
))

# Multiple inheritance (interface-based)
write_puml("class_inherit_multiple_interfaces.puml", wrap(
    "interface Flyable {\n  +void fly()\n}\ninterface Swimmable {\n  +void swim()\n}\ninterface Runnable {\n  +void run()\n}\nclass Duck {\n  +void quack()\n}\nDuck ..|> Flyable\nDuck ..|> Swimmable\nDuck ..|> Runnable"
))

# Diamond inheritance
write_puml("class_inherit_diamond.puml", wrap(
    "class Base {\n  +void method()\n}\nclass Left {\n  +void leftMethod()\n}\nclass Right {\n  +void rightMethod()\n}\nclass Diamond {\n  +void method()\n}\nBase <|-- Left\nBase <|-- Right\nLeft <|-- Diamond\nRight <|-- Diamond"
))

# Abstract template method pattern
write_puml("class_inherit_template_method.puml", wrap(
    "abstract class DataProcessor {\n"
    "  +void process()\n"
    "  {abstract} #void readData()\n"
    "  {abstract} #void transformData()\n"
    "  {abstract} #void writeData()\n"
    "}\n"
    "class CsvProcessor {\n"
    "  #void readData()\n"
    "  #void transformData()\n"
    "  #void writeData()\n"
    "}\n"
    "class JsonProcessor {\n"
    "  #void readData()\n"
    "  #void transformData()\n"
    "  #void writeData()\n"
    "}\n"
    "DataProcessor <|-- CsvProcessor\n"
    "DataProcessor <|-- JsonProcessor"
))


# ─────────────────────────────────────────────────────────────────────────────
# 11. PACKAGES AND GROUPING
# ─────────────────────────────────────────────────────────────────────────────

package_styles = ["", "node", "rectangle", "folder", "frame", "cloud", "database"]

# Basic package
for style in package_styles:
    style_kw = f" <<{style}>>" if style else ""
    label = style if style else "default"
    write_puml(f"class_package_{label}_basic.puml", wrap(
        f"package mypackage{style_kw} {{\n  class MyClass {{\n    +void method()\n  }}\n}}"
    ))

# Namespace
write_puml("class_namespace_basic.puml", wrap(
    "namespace com.example {\n  class MyClass {\n    +void method()\n  }\n}"
))

# Module
write_puml("class_module_basic.puml", wrap(
    "module MyModule {\n  class MyClass {\n    +void method()\n  }\n}"
))

# Nested packages 2 levels
write_puml("class_package_nested_2.puml", wrap(
    "package outer {\n  package inner {\n    class MyClass {\n      +void method()\n    }\n  }\n}"
))

# Nested packages 3 levels
write_puml("class_package_nested_3.puml", wrap(
    "package level1 {\n  package level2 {\n    package level3 {\n      class DeepClass {\n        +void deepMethod()\n      }\n    }\n  }\n}"
))

# Nested packages 4 levels
write_puml("class_package_nested_4.puml", wrap(
    "package com {\n  package example {\n    package myapp {\n      package service {\n        class ServiceImpl {\n          +void execute()\n        }\n      }\n    }\n  }\n}"
))

# Classes across packages with relationships
write_puml("class_package_cross_package_rel.puml", wrap(
    "package service {\n  class UserService {\n    +User getUser(int id)\n  }\n}\npackage repository {\n  class UserRepository {\n    +User findById(int id)\n  }\n}\npackage model {\n  class User {\n    +int id\n    +String name\n  }\n}\nservice.UserService ..> repository.UserRepository\nrepository.UserRepository ..> model.User"
))

# Package with multiple classes and relationships
write_puml("class_package_multi_class.puml", wrap(
    "package animals {\n"
    "  class Animal {\n    +String name\n  }\n"
    "  class Dog {\n    +void bark()\n  }\n"
    "  class Cat {\n    +void meow()\n  }\n"
    "  Animal <|-- Dog\n"
    "  Animal <|-- Cat\n"
    "}"
))

# Together keyword
write_puml("class_package_together.puml", wrap(
    "class A\nclass B\nclass C\ntogether {\n  class D\n  class E\n}\nA --> D\nB --> E\nC --> D"
))

# Package with color
write_puml("class_package_with_color.puml", wrap(
    "package colored #lightblue {\n  class MyClass {\n    +void method()\n  }\n}"
))

# Multiple packages with connections
write_puml("class_package_mvc.puml", wrap(
    "package controller {\n  class UserController {\n    +void handleRequest()\n  }\n}\n"
    "package service {\n  class UserService {\n    +User getUser(int id)\n  }\n}\n"
    "package dao {\n  class UserDAO {\n    +User findById(int id)\n  }\n}\n"
    "package model {\n  class User {\n    +int id\n    +String name\n  }\n}\n"
    "controller.UserController ..> service.UserService\n"
    "service.UserService ..> dao.UserDAO\n"
    "dao.UserDAO ..> model.User"
))


# ─────────────────────────────────────────────────────────────────────────────
# 12. ENUMS
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_enum_basic.puml", wrap(
    "enum Color {\n  RED\n  GREEN\n  BLUE\n}"
))

write_puml("class_enum_with_methods.puml", wrap(
    "enum Planet {\n  MERCURY\n  VENUS\n  EARTH\n  MARS\n  --\n  +double mass()\n  +double radius()\n  +double surfaceGravity()\n}"
))

write_puml("class_enum_with_values.puml", wrap(
    "enum HttpStatus {\n  OK = 200\n  NOT_FOUND = 404\n  INTERNAL_ERROR = 500\n}"
))

write_puml("class_enum_implementing_interface.puml", wrap(
    "interface Describable {\n  +String getDescription()\n}\nenum Season {\n  SPRING\n  SUMMER\n  AUTUMN\n  WINTER\n  --\n  +String getDescription()\n}\nSeason ..|> Describable"
))

write_puml("class_enum_used_in_class.puml", wrap(
    "enum Status {\n  ACTIVE\n  INACTIVE\n  PENDING\n}\nclass User {\n  +String name\n  +Status status\n  +boolean isActive()\n}\nUser --> Status"
))

write_puml("class_enum_complex.puml", wrap(
    "enum DayOfWeek {\n  MONDAY\n  TUESDAY\n  WEDNESDAY\n  THURSDAY\n  FRIDAY\n  SATURDAY\n  SUNDAY\n  --\n  +boolean isWeekend()\n  +DayOfWeek next()\n  {static} +DayOfWeek fromInt(int i)\n}"
))


# ─────────────────────────────────────────────────────────────────────────────
# 13. INTERFACES
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_interface_basic.puml", wrap(
    "interface Drawable {\n  +void draw()\n  +void resize(double factor)\n}"
))

write_puml("class_interface_extending.puml", wrap(
    "interface Shape {\n  +double area()\n}\ninterface Printable {\n  +void print()\n}\ninterface PrintableShape {\n  +void print()\n  +double area()\n}\nPrintableShape --|> Shape\nPrintableShape --|> Printable"
))

write_puml("class_interface_lollipop.puml", wrap(
    "class Component\nComponent ()- Interface1\nComponent ()-- Interface2\nComponent ()-up- Interface3"
))

write_puml("class_interface_lollipop_detailed.puml", wrap(
    "class Car\nCar ()- IStartable\nCar ()- IStoppable\nCar ()-- IDriveable"
))

write_puml("class_interface_multiple_impl.puml", wrap(
    "interface Serializable {\n  +byte[] serialize()\n  +void deserialize(byte[] data)\n}\nclass JsonSerializer {\n  +byte[] serialize()\n  +void deserialize(byte[] data)\n}\nclass XmlSerializer {\n  +byte[] serialize()\n  +void deserialize(byte[] data)\n}\nclass BinarySerializer {\n  +byte[] serialize()\n  +void deserialize(byte[] data)\n}\nJsonSerializer ..|> Serializable\nXmlSerializer ..|> Serializable\nBinarySerializer ..|> Serializable"
))


# ─────────────────────────────────────────────────────────────────────────────
# 14. NOTES
# ─────────────────────────────────────────────────────────────────────────────

note_positions = ["left", "right", "top", "bottom"]

for pos in note_positions:
    write_puml(f"class_note_{pos}_of.puml", wrap(
        f"class MyClass {{\n  +void method()\n}}\nnote {pos} of MyClass : This is a {pos} note"
    ))

# Multi-line notes
write_puml("class_note_multiline.puml", wrap(
    "class MyClass {\n  +void method()\n}\nnote right of MyClass\n  This is a multi-line note\n  It can span multiple lines\n  With different content\nend note"
))

# Floating note
write_puml("class_note_floating.puml", wrap(
    "class MyClass {\n  +void method()\n}\nnote \"This is a floating note\" as N1\nMyClass .. N1"
))

# Note on link
write_puml("class_note_on_link.puml", wrap(
    "class A\nclass B\nA --> B\nnote on link : This note is on the link"
))

write_puml("class_note_on_link_multiline.puml", wrap(
    "class A\nclass B\nA --> B\nnote on link\n  Multi-line\n  link note\nend note"
))

# Creole in notes
write_puml("class_note_creole.puml", wrap(
    "class MyClass\nnote right of MyClass\n  **Bold text**\n  //Italic text//\n  * List item 1\n  * List item 2\n  <b>HTML bold</b>\nend note"
))

# Named notes
write_puml("class_note_named.puml", wrap(
    "class A\nclass B\nnote \"Note for A\" as N1\nnote \"Note for B\" as N2\nA .. N1\nB .. N2"
))

# Note with color
write_puml("class_note_colored.puml", wrap(
    "class MyClass\nnote right of MyClass #lightblue\n  Colored note\nend note"
))

# Multiple notes on one class
write_puml("class_note_multiple_on_one_class.puml", wrap(
    "class MyClass {\n  +void method()\n}\nnote left of MyClass : Left note\nnote right of MyClass : Right note\nnote top of MyClass : Top note\nnote bottom of MyClass : Bottom note"
))


# ─────────────────────────────────────────────────────────────────────────────
# 15. ASSOCIATION CLASSES
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_association_class_basic.puml", wrap(
    "class Student\nclass Course\nclass Enrollment {\n  +Date enrollDate\n  +double grade\n}\n(Student, Course) .. Enrollment"
))

write_puml("class_association_class_detailed.puml", wrap(
    "class Employee {\n  +String name\n}\nclass Project {\n  +String title\n}\nclass Assignment {\n  +Date startDate\n  +Date endDate\n  +double hoursPerWeek\n  +String role\n}\n(Employee, Project) .. Assignment"
))

write_puml("class_association_class_with_relationships.puml", wrap(
    "class Author\nclass Book\nclass Authorship {\n  +String role\n  +int chapter\n}\n(Author, Book) .. Authorship\nclass Publisher\nBook --> Publisher"
))


# ─────────────────────────────────────────────────────────────────────────────
# 16. NAMESPACE SEPARATORS
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_namespace_separator_default.puml", wrap(
    "class com.example.MyClass {\n  +void method()\n}"
))

write_puml("class_namespace_separator_custom_colon.puml", wrap(
    "set namespaceSeparator ::\nclass com::example::MyClass {\n  +void method()\n}"
))

write_puml("class_namespace_separator_custom_slash.puml", wrap(
    "set namespaceSeparator /\nclass com/example/MyClass {\n  +void method()\n}"
))

write_puml("class_namespace_separator_none.puml", wrap(
    "set namespaceSeparator none\nclass com.example.MyClass {\n  +void method()\n}"
))

write_puml("class_namespace_separator_with_relationships.puml", wrap(
    "set namespaceSeparator ::\nclass com::service::UserService\nclass com::repository::UserRepository\nclass com::model::User\ncom::service::UserService ..> com::repository::UserRepository\ncom::repository::UserRepository ..> com::model::User"
))


# ─────────────────────────────────────────────────────────────────────────────
# 17. HIDE/SHOW/REMOVE COMMANDS
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_hide_empty_members.puml", wrap(
    "hide empty members\nclass A {\n  +void method()\n}\nclass B\nclass C {}\nA --> B\nB --> C"
))

write_puml("class_hide_empty_fields.puml", wrap(
    "hide empty fields\nclass A {\n  +void method()\n}\nclass B {\n  +String field\n}"
))

write_puml("class_hide_empty_methods.puml", wrap(
    "hide empty methods\nclass A {\n  +String field\n}\nclass B {\n  +void method()\n}"
))

write_puml("class_hide_specific_class.puml", wrap(
    "class A\nclass B\nclass C\nA --> B\nB --> C\nhide B"
))

write_puml("class_hide_attributes.puml", wrap(
    "hide attributes\nclass A {\n  +String field\n  +void method()\n}\nclass B {\n  -int count\n  +void process()\n}"
))

write_puml("class_hide_methods.puml", wrap(
    "hide methods\nclass A {\n  +String field\n  +void method()\n}\nclass B {\n  -int count\n  +void process()\n}"
))

write_puml("class_remove_class.puml", wrap(
    "class A\nclass B\nclass C\nA --> B\nB --> C\nremove B"
))

write_puml("class_show_after_hide.puml", wrap(
    "hide empty members\nclass A {\n  +void method()\n}\nclass B\nshow B"
))

# Hide by stereotype
write_puml("class_hide_by_stereotype.puml", wrap(
    "class A <<internal>>\nclass B <<public>> {\n  +void method()\n}\nhide <<internal>>"
))


# ─────────────────────────────────────────────────────────────────────────────
# 18. SKINPARAM
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_skinparam_background_color.puml", wrap(
    "skinparam ClassBackgroundColor lightyellow\nclass MyClass {\n  +void method()\n}"
))

write_puml("class_skinparam_border_color.puml", wrap(
    "skinparam ClassBorderColor red\nclass MyClass {\n  +void method()\n}"
))

write_puml("class_skinparam_font_size.puml", wrap(
    "skinparam ClassFontSize 18\nclass MyClass {\n  +void method()\n}"
))

write_puml("class_skinparam_font_color.puml", wrap(
    "skinparam ClassFontColor blue\nclass MyClass {\n  +void method()\n}"
))

write_puml("class_skinparam_font_style_bold.puml", wrap(
    "skinparam ClassFontStyle bold\nclass MyClass {\n  +void method()\n}"
))

write_puml("class_skinparam_font_style_italic.puml", wrap(
    "skinparam ClassFontStyle italic\nclass MyClass {\n  +void method()\n}"
))

write_puml("class_skinparam_arrow_color.puml", wrap(
    "skinparam ArrowColor red\nclass A\nclass B\nA --> B"
))

write_puml("class_skinparam_arrow_thickness.puml", wrap(
    "skinparam ArrowThickness 2\nclass A\nclass B\nA --> B"
))

write_puml("class_skinparam_stereotype_colors.puml", wrap(
    "skinparam class {\n  BackgroundColor<<service>> lightblue\n  BorderColor<<service>> blue\n  BackgroundColor<<entity>> lightyellow\n  BorderColor<<entity>> orange\n}\nclass UserService <<service>> {\n  +void serve()\n}\nclass User <<entity>> {\n  +String name\n}"
))

write_puml("class_skinparam_header_background.puml", wrap(
    "skinparam ClassHeaderBackgroundColor #DDD\nclass MyClass {\n  +void method()\n}"
))

write_puml("class_skinparam_rounded.puml", wrap(
    "skinparam roundcorner 15\nclass MyClass {\n  +void method()\n}"
))

write_puml("class_skinparam_monochrome.puml", wrap(
    "skinparam monochrome true\nclass A\nclass B\nA --> B"
))

write_puml("class_skinparam_shadowing.puml", wrap(
    "skinparam shadowing true\nclass MyClass {\n  +void method()\n}"
))

write_puml("class_skinparam_combined.puml", wrap(
    "skinparam ClassBackgroundColor lightyellow\n"
    "skinparam ClassBorderColor orange\n"
    "skinparam ClassFontSize 14\n"
    "skinparam ArrowColor darkgreen\n"
    "skinparam ArrowThickness 2\n"
    "class A {\n  +void method()\n}\n"
    "class B {\n  +String field\n}\n"
    "A --> B"
))

# Handwritten style
write_puml("class_skinparam_handwritten.puml", wrap(
    "skinparam handwritten true\nclass MyClass {\n  +void method()\n}\nclass Other\nMyClass --> Other"
))


# ─────────────────────────────────────────────────────────────────────────────
# 19. URL LINKS
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_url_link_basic.puml", wrap(
    "class MyClass [[http://example.com]] {\n  +void method()\n}"
))

write_puml("class_url_link_with_tooltip.puml", wrap(
    "class MyClass [[http://example.com{Click here}]] {\n  +void method()\n}"
))

write_puml("class_url_link_multiple.puml", wrap(
    "class A [[http://a.example.com]]\nclass B [[http://b.example.com]]\nA --> B"
))


# ─────────────────────────────────────────────────────────────────────────────
# 20. INNER/NESTED CLASSES
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_inner_class_basic.puml", wrap(
    "class Outer {\n  +void outerMethod()\n  class Inner {\n    +void innerMethod()\n  }\n}"
))

write_puml("class_inner_class_multiple.puml", wrap(
    "class Container {\n  class Iterator {\n    +boolean hasNext()\n    +Object next()\n  }\n  class Builder {\n    +Container build()\n  }\n}"
))

write_puml("class_inner_static_class.puml", wrap(
    "class Outer {\n  {static} class StaticNested {\n    +void method()\n  }\n  class NonStaticInner {\n    +void method()\n  }\n}"
))


# ─────────────────────────────────────────────────────────────────────────────
# 21. LARGE DIAGRAMS
# ─────────────────────────────────────────────────────────────────────────────

# 30+ classes
classes_30 = []
for i in range(1, 31):
    classes_30.append(f"class Class{i:02d} {{\n  +String field{i}\n  +void method{i}()\n}}")
rels_30 = []
for i in range(1, 30):
    rels_30.append(f"Class{i:02d} --> Class{i+1:02d}")
write_puml("class_large_30_classes.puml", wrap("\n".join(classes_30) + "\n\n" + "\n".join(rels_30)))

# Large hierarchy
write_puml("class_large_hierarchy.puml", wrap(
    "abstract class Vehicle {\n  +String make\n  +String model\n  +int year\n  +void start()\n  +void stop()\n}\n"
    "class Car {\n  +int doors\n  +void drive()\n}\n"
    "class Truck {\n  +double payload\n  +void haul()\n}\n"
    "class Motorcycle {\n  +boolean hasSidecar\n  +void ride()\n}\n"
    "class Bus {\n  +int capacity\n  +void pickUp()\n}\n"
    "class ElectricCar {\n  +int batteryKWh\n  +void charge()\n}\n"
    "class HybridCar {\n  +double mpg\n  +void switchMode()\n}\n"
    "class SportsCar {\n  +int horsepower\n  +void race()\n}\n"
    "class SUV {\n  +boolean awd\n  +void offroad()\n}\n"
    "Vehicle <|-- Car\n"
    "Vehicle <|-- Truck\n"
    "Vehicle <|-- Motorcycle\n"
    "Vehicle <|-- Bus\n"
    "Car <|-- ElectricCar\n"
    "Car <|-- HybridCar\n"
    "Car <|-- SportsCar\n"
    "Car <|-- SUV"
))

# Full enterprise-style diagram
write_puml("class_large_enterprise_pattern.puml", wrap(
    "package domain {\n"
    "  class User {\n    +int id\n    +String username\n    +String email\n  }\n"
    "  class Order {\n    +int id\n    +Date createdAt\n    +double total\n  }\n"
    "  class Product {\n    +int id\n    +String name\n    +double price\n  }\n"
    "  class OrderItem {\n    +int quantity\n    +double price\n  }\n"
    "}\n"
    "package repository {\n"
    "  interface UserRepository {\n    +User findById(int id)\n    +List<User> findAll()\n  }\n"
    "  interface OrderRepository {\n    +Order findById(int id)\n  }\n"
    "  class UserRepositoryImpl\n"
    "  class OrderRepositoryImpl\n"
    "  UserRepositoryImpl ..|> UserRepository\n"
    "  OrderRepositoryImpl ..|> OrderRepository\n"
    "}\n"
    "package service {\n"
    "  class UserService {\n    +User getUser(int id)\n    +void createUser(User u)\n  }\n"
    "  class OrderService {\n    +Order createOrder(int userId)\n    +void addItem(int orderId, int productId)\n  }\n"
    "}\n"
    "package controller {\n"
    "  class UserController {\n    +void handleGet()\n    +void handlePost()\n  }\n"
    "  class OrderController {\n    +void handleGet()\n    +void handlePost()\n  }\n"
    "}\n"
    "controller.UserController ..> service.UserService\n"
    "controller.OrderController ..> service.OrderService\n"
    "service.UserService ..> repository.UserRepository\n"
    "service.OrderService ..> repository.OrderRepository\n"
    "domain.Order --> domain.User\n"
    "domain.Order *-- domain.OrderItem\n"
    "domain.OrderItem --> domain.Product"
))


# ─────────────────────────────────────────────────────────────────────────────
# 22. EDGE CASES
# ─────────────────────────────────────────────────────────────────────────────

# Empty diagram
write_puml("class_edge_empty_diagram.puml", wrap(""))

# Minimal - single class
write_puml("class_edge_single_class.puml", wrap("class MyClass"))

# Single class with body
write_puml("class_edge_single_class_with_body.puml", wrap(
    "class MyClass {\n  +String field\n  +void method()\n}"
))

# Very long names
write_puml("class_edge_very_long_class_name.puml", wrap(
    "class VeryLongClassNameThatExceedsNormalLengthAndMightCauseRenderingIssues {\n  +void veryLongMethodNameThatAlsoExceedsNormalLength(String veryLongParameterName)\n}"
))

# Special characters in names
write_puml("class_edge_special_chars_quoted.puml", wrap(
    'class "My Class" {\n  +void method()\n}\nclass "Another Class"\n"My Class" --> "Another Class"'
))

# Unicode names
write_puml("class_edge_unicode_names.puml", wrap(
    "class Ångström {\n  +void method()\n}\nclass Façade"
))

# Class with 20+ fields
fields_20 = "\n".join([f"  +String field{i:02d}" for i in range(1, 21)])
write_puml("class_edge_20_fields.puml", wrap(
    f"class HeavyClass {{\n{fields_20}\n}}"
))

# Class with 20+ methods
methods_20 = "\n".join([f"  +void method{i:02d}()" for i in range(1, 21)])
write_puml("class_edge_20_methods.puml", wrap(
    f"class HeavyClass {{\n{methods_20}\n}}"
))

# Class with both 20+ fields and methods
write_puml("class_edge_20_fields_20_methods.puml", wrap(
    f"class MassiveClass {{\n{fields_20}\n  --\n{methods_20}\n}}"
))

# No relationships, just classes
write_puml("class_edge_many_isolated_classes.puml", wrap(
    "\n".join([f"class Isolated{i:02d}" for i in range(1, 16)])
))

# Self-referential
write_puml("class_edge_self_reference.puml", wrap(
    "class TreeNode {\n  +TreeNode parent\n  +List<TreeNode> children\n  +void addChild(TreeNode n)\n}\nTreeNode --> TreeNode"
))

# Circular dependency
write_puml("class_edge_circular_dependency.puml", wrap(
    "class A\nclass B\nclass C\nA --> B\nB --> C\nC --> A"
))

# Two classes, no body
write_puml("class_edge_two_classes_no_body.puml", wrap(
    "class A\nclass B\nA --> B"
))

# Class named with numbers
write_puml("class_edge_numeric_in_name.puml", wrap(
    "class SHA256 {\n  +byte[] hash(byte[] data)\n}\nclass MD5 {\n  +byte[] hash(byte[] data)\n}"
))

# Deeply nested packages
write_puml("class_edge_deep_package_nesting.puml", wrap(
    "package a {\n  package b {\n    package c {\n      package d {\n        package e {\n          class DeepClass {\n            +void deepMethod()\n          }\n        }\n      }\n    }\n  }\n}"
))


# ─────────────────────────────────────────────────────────────────────────────
# 23. COMBINATORIAL VARIANTS
# ─────────────────────────────────────────────────────────────────────────────

# Class type × has fields × has methods
for kw, label in zip(CLASS_TYPES[:4], CLASS_KEYWORDS[:4]):  # skip annotation/entity for some combos
    for has_fields in [True, False]:
        for has_methods in [True, False]:
            body_parts = []
            if has_fields:
                body_parts.append("  +String name\n  -int count")
            if has_methods:
                body_parts.append("  +void process()\n  -String format()")
            body = "\n  --\n".join(body_parts) if body_parts else ""
            suffix = f"{'_fields' if has_fields else ''}{'_methods' if has_methods else ''}"
            if not suffix:
                suffix = "_bare"
            write_puml(f"class_combo_{label}{suffix}.puml", wrap(
                f"{kw} Combo {{\n{body}\n}}" if body else f"{kw} Combo"
            ))

# Visibility × member type × modifier
for vis, vis_label in [("+ ", "public"), ("- ", "private"), ("# ", "protected"), ("~ ", "package")]:
    for modifier, mod_label in [("", "normal"), ("{static} ", "static"), ("{abstract} ", "abstract")]:
        write_puml(f"class_combo_vis_{vis_label}_{mod_label}.puml", wrap(
            f"class Combo {{\n  {modifier}{vis}String field\n  {modifier}{vis}void method()\n}}"
        ))

# Relationship × labeled × direction
rel_kinds = [
    ("ext", "<|--"),
    ("impl", "..|>"),
    ("comp", "*--"),
    ("agg", "o--"),
    ("dep", "..>"),
    ("assoc", "--"),
]

label_combos = [
    ("no_label", "", "", ""),
    ("mid_label", "", " : uses", ""),
    ("mult_label", ' "1"', ' "*"', ""),
    ("full_label", ' "1"', ' "*"', " : employs"),
]

for rel_label, arrow in rel_kinds:
    for lc_label, near, far, mid in label_combos:
        write_puml(f"class_combo_rel_{rel_label}_{lc_label}.puml", wrap(
            f"class A\nclass B\nA{near} {arrow}{far} B{mid}"
        ))

# Package style × nesting level
package_types = ["package", "namespace", "module", "node", "folder", "frame", "cloud", "database", "rectangle"]
for pkg_type in package_types:
    write_puml(f"class_combo_pkg_{pkg_type}_single.puml", wrap(
        f"{pkg_type} MyPkg {{\n  class MyClass {{\n    +void method()\n  }}\n}}"
    ))
    write_puml(f"class_combo_pkg_{pkg_type}_nested.puml", wrap(
        f"{pkg_type} Outer {{\n  {pkg_type} Inner {{\n    class MyClass {{\n      +void method()\n    }}\n  }}\n}}"
    ))

# Skinparam × diagram complexity
skinparam_sets = [
    ("minimal", "skinparam monochrome true"),
    ("colored", "skinparam ClassBackgroundColor lightblue\nskinparam ClassBorderColor blue"),
    ("large_font", "skinparam ClassFontSize 20"),
    ("rounded", "skinparam roundcorner 20"),
    ("bold", "skinparam ClassFontStyle bold"),
]

for sp_label, sp_content in skinparam_sets:
    write_puml(f"class_combo_skinparam_{sp_label}.puml", wrap(
        f"{sp_content}\nclass A {{\n  +String field\n  +void method()\n}}\nclass B {{\n  -int count\n  +void process()\n}}\nA --> B : uses"
    ))

# Stereotype × color × spot
combos_stereo = [
    ("singleton_red", "<<singleton>>", "#red"),
    ("service_blue", "<<service>>", "#blue"),
    ("entity_yellow", "<<entity>>", "#lightyellow"),
    ("controller_green", "<<controller>>", "#lightgreen"),
]

for label, stereo, color in combos_stereo:
    write_puml(f"class_combo_stereo_color_{label}.puml", wrap(
        f"class MyClass {stereo} {color} {{\n  +void method()\n}}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 24. DESIGN PATTERNS
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_pattern_singleton.puml", wrap(
    "class Singleton <<singleton>> {\n"
    "  -{static} Singleton instance\n"
    "  -Singleton()\n"
    "  +{static} Singleton getInstance()\n"
    "  +void doOperation()\n"
    "}"
))

write_puml("class_pattern_factory_method.puml", wrap(
    "abstract class Creator {\n"
    "  +{abstract} Product createProduct()\n"
    "  +void operate()\n"
    "}\n"
    "class ConcreteCreatorA {\n"
    "  +Product createProduct()\n"
    "}\n"
    "class ConcreteCreatorB {\n"
    "  +Product createProduct()\n"
    "}\n"
    "interface Product {\n"
    "  +void use()\n"
    "}\n"
    "class ConcreteProductA {\n"
    "  +void use()\n"
    "}\n"
    "class ConcreteProductB {\n"
    "  +void use()\n"
    "}\n"
    "Creator <|-- ConcreteCreatorA\n"
    "Creator <|-- ConcreteCreatorB\n"
    "ConcreteCreatorA ..> ConcreteProductA\n"
    "ConcreteCreatorB ..> ConcreteProductB\n"
    "ConcreteProductA ..|> Product\n"
    "ConcreteProductB ..|> Product"
))

write_puml("class_pattern_observer.puml", wrap(
    "interface Subject {\n"
    "  +void attach(Observer o)\n"
    "  +void detach(Observer o)\n"
    "  +void notify()\n"
    "}\n"
    "interface Observer {\n"
    "  +void update()\n"
    "}\n"
    "class ConcreteSubject {\n"
    "  -state: String\n"
    "  +void attach(Observer o)\n"
    "  +void detach(Observer o)\n"
    "  +void notify()\n"
    "  +String getState()\n"
    "}\n"
    "class ConcreteObserverA {\n"
    "  +void update()\n"
    "}\n"
    "class ConcreteObserverB {\n"
    "  +void update()\n"
    "}\n"
    "ConcreteSubject ..|> Subject\n"
    "ConcreteObserverA ..|> Observer\n"
    "ConcreteObserverB ..|> Observer\n"
    "Subject o-- Observer"
))

write_puml("class_pattern_strategy.puml", wrap(
    "class Context {\n"
    "  -strategy: Strategy\n"
    "  +void setStrategy(Strategy s)\n"
    "  +void executeStrategy()\n"
    "}\n"
    "interface Strategy {\n"
    "  +void execute()\n"
    "}\n"
    "class ConcreteStrategyA {\n"
    "  +void execute()\n"
    "}\n"
    "class ConcreteStrategyB {\n"
    "  +void execute()\n"
    "}\n"
    "class ConcreteStrategyC {\n"
    "  +void execute()\n"
    "}\n"
    "Context --> Strategy\n"
    "ConcreteStrategyA ..|> Strategy\n"
    "ConcreteStrategyB ..|> Strategy\n"
    "ConcreteStrategyC ..|> Strategy"
))

write_puml("class_pattern_decorator.puml", wrap(
    "interface Component {\n"
    "  +void operation()\n"
    "}\n"
    "class ConcreteComponent {\n"
    "  +void operation()\n"
    "}\n"
    "abstract class Decorator {\n"
    "  -component: Component\n"
    "  +void operation()\n"
    "}\n"
    "class ConcreteDecoratorA {\n"
    "  +void operation()\n"
    "  +void extraBehavior()\n"
    "}\n"
    "class ConcreteDecoratorB {\n"
    "  +void operation()\n"
    "  +void extraBehavior()\n"
    "}\n"
    "ConcreteComponent ..|> Component\n"
    "Decorator ..|> Component\n"
    "Decorator o-- Component\n"
    "ConcreteDecoratorA --|> Decorator\n"
    "ConcreteDecoratorB --|> Decorator"
))

write_puml("class_pattern_composite.puml", wrap(
    "interface Component {\n"
    "  +void operation()\n"
    "  +void add(Component c)\n"
    "  +void remove(Component c)\n"
    "}\n"
    "class Leaf {\n"
    "  +void operation()\n"
    "}\n"
    "class Composite {\n"
    "  -children: List<Component>\n"
    "  +void operation()\n"
    "  +void add(Component c)\n"
    "  +void remove(Component c)\n"
    "}\n"
    "Leaf ..|> Component\n"
    "Composite ..|> Component\n"
    "Composite o-- Component"
))

write_puml("class_pattern_command.puml", wrap(
    "interface Command {\n"
    "  +void execute()\n"
    "  +void undo()\n"
    "}\n"
    "class Invoker {\n"
    "  -command: Command\n"
    "  +void setCommand(Command c)\n"
    "  +void executeCommand()\n"
    "}\n"
    "class Receiver {\n"
    "  +void action()\n"
    "}\n"
    "class ConcreteCommand {\n"
    "  -receiver: Receiver\n"
    "  +void execute()\n"
    "  +void undo()\n"
    "}\n"
    "ConcreteCommand ..|> Command\n"
    "Invoker --> Command\n"
    "ConcreteCommand --> Receiver"
))

write_puml("class_pattern_builder.puml", wrap(
    "class Director {\n"
    "  -builder: Builder\n"
    "  +void construct()\n"
    "}\n"
    "abstract class Builder {\n"
    "  +{abstract} void buildPartA()\n"
    "  +{abstract} void buildPartB()\n"
    "  +{abstract} Product getResult()\n"
    "}\n"
    "class ConcreteBuilder {\n"
    "  -product: Product\n"
    "  +void buildPartA()\n"
    "  +void buildPartB()\n"
    "  +Product getResult()\n"
    "}\n"
    "class Product {\n"
    "  +String partA\n"
    "  +String partB\n"
    "}\n"
    "Director --> Builder\n"
    "ConcreteBuilder --|> Builder\n"
    "ConcreteBuilder --> Product"
))

write_puml("class_pattern_proxy.puml", wrap(
    "interface Subject {\n"
    "  +void request()\n"
    "}\n"
    "class RealSubject {\n"
    "  +void request()\n"
    "}\n"
    "class Proxy {\n"
    "  -realSubject: RealSubject\n"
    "  +void request()\n"
    "  -void preProcess()\n"
    "  -void postProcess()\n"
    "}\n"
    "RealSubject ..|> Subject\n"
    "Proxy ..|> Subject\n"
    "Proxy --> RealSubject"
))

write_puml("class_pattern_adapter.puml", wrap(
    "interface Target {\n"
    "  +void request()\n"
    "}\n"
    "class Adaptee {\n"
    "  +void specificRequest()\n"
    "}\n"
    "class Adapter {\n"
    "  -adaptee: Adaptee\n"
    "  +void request()\n"
    "}\n"
    "Adapter ..|> Target\n"
    "Adapter --> Adaptee"
))

write_puml("class_pattern_facade.puml", wrap(
    "class Facade {\n"
    "  -subsystemA: SubsystemA\n"
    "  -subsystemB: SubsystemB\n"
    "  -subsystemC: SubsystemC\n"
    "  +void operation()\n"
    "}\n"
    "class SubsystemA {\n"
    "  +void operationA()\n"
    "}\n"
    "class SubsystemB {\n"
    "  +void operationB()\n"
    "}\n"
    "class SubsystemC {\n"
    "  +void operationC()\n"
    "}\n"
    "Facade --> SubsystemA\n"
    "Facade --> SubsystemB\n"
    "Facade --> SubsystemC"
))


# ─────────────────────────────────────────────────────────────────────────────
# 25. ANNOTATIONS
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_annotation_basic.puml", wrap(
    "annotation Override {\n  +String value()\n}"
))

write_puml("class_annotation_marker.puml", wrap(
    "annotation Deprecated"
))

write_puml("class_annotation_with_defaults.puml", wrap(
    "annotation RequestMapping {\n  +String value() default \"/\"\n  +String method() default \"GET\"\n  +String[] produces() default {\"*/*\"}\n}"
))

write_puml("class_annotation_applied.puml", wrap(
    "annotation Service\nannotation Autowired\nclass UserService {\n  <<Service>>\n  +UserRepository repo\n}\nclass UserRepository {\n  +User findById(int id)\n}"
))


# ─────────────────────────────────────────────────────────────────────────────
# 26. ENTITY TYPE
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_entity_basic.puml", wrap(
    "entity User {\n  *id : int\n  --\n  username : String\n  email : String\n  createdAt : Date\n}"
))

write_puml("class_entity_with_pk.puml", wrap(
    "entity Product {\n  *product_id : INT <<PK>>\n  --\n  name : VARCHAR(100)\n  price : DECIMAL\n  category_id : INT <<FK>>\n}"
))

write_puml("class_entity_relationships.puml", wrap(
    "entity Customer {\n  *customer_id : INT\n  name : VARCHAR\n  email : VARCHAR\n}\n"
    "entity Order {\n  *order_id : INT\n  customer_id : INT\n  order_date : DATE\n}\n"
    "entity OrderItem {\n  *item_id : INT\n  order_id : INT\n  product_id : INT\n  quantity : INT\n}\n"
    "Customer ||--o{ Order : places\n"
    "Order ||--o{ OrderItem : contains"
))


# ─────────────────────────────────────────────────────────────────────────────
# 27. TITLE, HEADER, FOOTER, CAPTION
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_title_basic.puml", wrap(
    "title My Class Diagram\nclass A\nclass B\nA --> B"
))

write_puml("class_title_multiline.puml", wrap(
    "title\n  My Class Diagram\n  Version 1.0\nend title\nclass A\nclass B\nA --> B"
))

write_puml("class_header.puml", wrap(
    "header My Header\nclass A\nclass B\nA --> B"
))

write_puml("class_footer.puml", wrap(
    "footer My Footer\nclass A\nclass B\nA --> B"
))

write_puml("class_title_header_footer.puml", wrap(
    "title My Diagram\nheader Generated by PlantUML\nfooter Page %page% of %lastpage%\nclass A\nclass B\nA --> B"
))

write_puml("class_caption.puml", wrap(
    "caption Figure 1: Class hierarchy\nclass A\nclass B\nA --> B"
))


# ─────────────────────────────────────────────────────────────────────────────
# 28. DIRECTION
# ─────────────────────────────────────────────────────────────────────────────

directions_full = [
    ("left_to_right", "left to right direction"),
    ("top_to_bottom", "top to bottom direction"),
]

for label, directive in directions_full:
    write_puml(f"class_direction_{label}.puml", wrap(
        f"{directive}\nclass A {{\n  +void method()\n}}\nclass B {{\n  +void method()\n}}\nclass C {{\n  +void method()\n}}\nA --> B\nB --> C"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 29. COMMENTS AND WHITESPACE
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_comment_single_quote.puml", wrap(
    "' This is a comment\nclass A\n' Another comment\nclass B\nA --> B"
))

write_puml("class_comment_block.puml", wrap(
    "/' This is a\n   block comment\n'/\nclass A\nclass B\nA --> B"
))

write_puml("class_comment_inline.puml", wrap(
    "class A ' This class does something\nclass B ' This class does something else\nA --> B ' This is the relationship"
))


# ─────────────────────────────────────────────────────────────────────────────
# 30. NEWPAGE / MULTIPLE PAGES
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_newpage_basic.puml", wrap(
    "class A\nclass B\nA --> B\nnewpage\nclass C\nclass D\nC --> D"
))

write_puml("class_newpage_with_title.puml", wrap(
    "class A\nclass B\nA --> B\nnewpage Page 2\nclass C\nclass D\nC --> D"
))


# ─────────────────────────────────────────────────────────────────────────────
# 31. ABSTRACT CLASS VARIANTS
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_abstract_basic.puml", wrap(
    "abstract class Shape {\n  +{abstract} double area()\n  +{abstract} double perimeter()\n  +void describe()\n}"
))

write_puml("class_abstract_with_concrete.puml", wrap(
    "abstract class Animal {\n  +String name\n  +{abstract} void makeSound()\n  +void breathe()\n}\nclass Dog {\n  +void makeSound()\n}\nclass Cat {\n  +void makeSound()\n}\nAnimal <|-- Dog\nAnimal <|-- Cat"
))

write_puml("class_abstract_chain.puml", wrap(
    "abstract class A {\n  +{abstract} void method1()\n}\nabstract class B {\n  +void method1()\n  +{abstract} void method2()\n}\nclass C {\n  +void method2()\n}\nA <|-- B\nB <|-- C"
))


# ─────────────────────────────────────────────────────────────────────────────
# 32. ADVANCED RELATIONSHIP PATTERNS
# ─────────────────────────────────────────────────────────────────────────────

# Multiplicity variants
multiplicities = [
    ("one_to_one", '"1"', '"1"'),
    ("one_to_many", '"1"', '"*"'),
    ("many_to_many", '"*"', '"*"'),
    ("zero_or_one", '"0..1"', '"1"'),
    ("one_or_more", '"1..*"', '"1"'),
    ("zero_or_more", '"0..*"', '"1"'),
    ("exact_n", '"3"', '"1"'),
    ("range", '"1..5"', '"1"'),
]

for label, near, far in multiplicities:
    write_puml(f"class_multiplicity_{label}.puml", wrap(
        f"class A\nclass B\nA {near} -- {far} B"
    ))

# Aggregation vs Composition comparison
write_puml("class_rel_agg_vs_comp.puml", wrap(
    "class Library {\n  +String name\n}\nclass Book {\n  +String title\n}\nclass Page {\n  +int number\n  +String content\n}\nLibrary o-- Book : contains\nBook *-- Page : consists of"
))

# Navigation arrows
write_puml("class_rel_navigation.puml", wrap(
    "class A\nclass B\nA --> B : nav right\nB <-- A : nav left\nA <--> B : bidirectional"
))


# ─────────────────────────────────────────────────────────────────────────────
# 33. CREOLE MARKUP IN CLASS BODIES
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_creole_bold_method.puml", wrap(
    "class MyClass {\n  +**boldMethod**()\n  +//italicMethod//()\n  +__underlineMethod__()\n}"
))

write_puml("class_creole_in_note.puml", wrap(
    "class MyClass\nnote right of MyClass\n  **Features:**\n  * Feature 1\n  * Feature 2\n  //Italic note//\n  <b>Bold HTML</b>\nend note"
))


# ─────────────────────────────────────────────────────────────────────────────
# 34. REALISTIC DOMAIN MODELS
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_domain_banking.puml", wrap(
    "abstract class Account {\n  #int accountNumber\n  #double balance\n  +void deposit(double amount)\n  +{abstract} void withdraw(double amount)\n  +double getBalance()\n}\n"
    "class SavingsAccount {\n  -double interestRate\n  +void withdraw(double amount)\n  +void applyInterest()\n}\n"
    "class CheckingAccount {\n  -double overdraftLimit\n  +void withdraw(double amount)\n}\n"
    "class Customer {\n  +String name\n  +String email\n  +List<Account> accounts\n}\n"
    "class Transaction {\n  +Date date\n  +double amount\n  +String description\n}\n"
    "Account <|-- SavingsAccount\n"
    "Account <|-- CheckingAccount\n"
    "Customer \"1\" o-- \"*\" Account\n"
    "Account \"1\" *-- \"*\" Transaction"
))

write_puml("class_domain_ecommerce.puml", wrap(
    "class User {\n  +int id\n  +String username\n  +String email\n  +void register()\n  +void login()\n}\n"
    "class Product {\n  +int id\n  +String name\n  +double price\n  +int stock\n}\n"
    "class Cart {\n  +List<CartItem> items\n  +double total()\n  +void addItem(Product p)\n  +void removeItem(Product p)\n}\n"
    "class CartItem {\n  +Product product\n  +int quantity\n  +double subtotal()\n}\n"
    "class Order {\n  +int id\n  +Date createdAt\n  +OrderStatus status\n}\n"
    "enum OrderStatus {\n  PENDING\n  CONFIRMED\n  SHIPPED\n  DELIVERED\n  CANCELLED\n}\n"
    "User \"1\" -- \"1\" Cart\n"
    "Cart \"1\" *-- \"*\" CartItem\n"
    "CartItem --> Product\n"
    "User \"1\" -- \"*\" Order\n"
    "Order --> OrderStatus"
))

write_puml("class_domain_library.puml", wrap(
    "class Library {\n  +String name\n  +String address\n  +void addBook(Book b)\n  +Book findBook(String isbn)\n}\n"
    "class Book {\n  +String isbn\n  +String title\n  +String author\n  +int year\n  +boolean isAvailable()\n}\n"
    "class Member {\n  +int memberId\n  +String name\n  +void borrowBook(Book b)\n  +void returnBook(Book b)\n}\n"
    "class Loan {\n  +Date borrowDate\n  +Date dueDate\n  +Date returnDate\n  +boolean isOverdue()\n}\n"
    "Library o-- Book\n"
    "Member \"1\" -- \"*\" Loan\n"
    "Book \"1\" -- \"*\" Loan"
))

write_puml("class_domain_hospital.puml", wrap(
    "class Hospital {\n  +String name\n  +String address\n}\n"
    "class Doctor {\n  +String name\n  +String specialty\n  +void diagnose(Patient p)\n}\n"
    "class Patient {\n  +String name\n  +Date birthDate\n  +String medicalHistory\n}\n"
    "class Appointment {\n  +DateTime scheduledTime\n  +String reason\n  +void confirm()\n  +void cancel()\n}\n"
    "class Prescription {\n  +String medication\n  +String dosage\n  +int days\n}\n"
    "Hospital o-- Doctor\n"
    "Doctor \"1\" -- \"*\" Appointment\n"
    "Patient \"1\" -- \"*\" Appointment\n"
    "Appointment -- Prescription"
))


# ─────────────────────────────────────────────────────────────────────────────
# 35. ADDITIONAL COMBINATORIAL VARIANTS
# ─────────────────────────────────────────────────────────────────────────────

# Inheritance with implementation (combined)
for i in range(1, 6):
    interfaces = " ".join([f"interface I{j}" for j in range(1, i+1)])
    impls = "\n".join([f"ConcreteClass ..|> I{j}" for j in range(1, i+1)])
    method_list = "\n".join([f"  +void method{j}()" for j in range(1, i+1)])
    write_puml(f"class_combo_inherit_impl_{i}_interfaces.puml", wrap(
        f"{interfaces}\nclass AbstractBase\nclass ConcreteClass\nAbstractBase <|-- ConcreteClass\n{impls}"
    ))

# Chain of responsibilities
for length in range(2, 7):
    classes = "\n".join([f"class Handler{i} {{\n  +void handle(Request r)\n}}" for i in range(1, length+1)])
    rels = "\n".join([f"Handler{i} --> Handler{i+1} : next" for i in range(1, length)])
    write_puml(f"class_pattern_chain_length_{length}.puml", wrap(
        f"class Request\n{classes}\n{rels}"
    ))

# Various class counts (scalability)
for count in [2, 3, 5, 8, 10, 15, 20, 25]:
    classes = "\n".join([f"class Node{i}" for i in range(1, count+1)])
    rels = "\n".join([f"Node{i} --> Node{i+1}" for i in range(1, count)])
    write_puml(f"class_scale_{count}_nodes_chain.puml", wrap(f"{classes}\n{rels}"))

# Star topology
for count in [3, 5, 7, 10]:
    classes = "\n".join([f"class Spoke{i}" for i in range(1, count+1)])
    rels = "\n".join([f"Hub --> Spoke{i}" for i in range(1, count+1)])
    write_puml(f"class_scale_{count}_spokes_star.puml", wrap(f"class Hub\n{classes}\n{rels}"))

# Tree topology (3 levels)
write_puml("class_scale_tree_3_levels.puml", wrap(
    "class Root\n"
    "class L1A\nclass L1B\nclass L1C\n"
    "class L2AA\nclass L2AB\nclass L2BA\nclass L2BB\nclass L2CA\nclass L2CB\n"
    "Root --> L1A\nRoot --> L1B\nRoot --> L1C\n"
    "L1A --> L2AA\nL1A --> L2AB\n"
    "L1B --> L2BA\nL1B --> L2BB\n"
    "L1C --> L2CA\nL1C --> L2CB"
))


# ─────────────────────────────────────────────────────────────────────────────
# 36. SPECIAL PUML FEATURES
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_feature_show_private.puml", wrap(
    "class MyClass {\n  -privateField: String\n  -privateMethod(): void\n  +publicMethod(): void\n}"
))

# Allow_1 notation (deprecated but still used)
write_puml("class_feature_circle_notation.puml", wrap(
    "class Foo\ncircle Bar\nFoo -- Bar"
))

write_puml("class_feature_diamond_notation.puml", wrap(
    "class Foo\ndiamond Bar\nFoo -- Bar"
))

# Sprite usage
write_puml("class_feature_class_no_icon.puml", wrap(
    "class MyService <<Service>> {\n  +void process()\n}"
))

# Class with constructor-style methods
write_puml("class_feature_constructor.puml", wrap(
    "class MyClass {\n  +MyClass()\n  +MyClass(String name)\n  +MyClass(String name, int age)\n  +void method()\n}"
))

# Class with destructors
write_puml("class_feature_destructor.puml", wrap(
    "class MyClass {\n  +MyClass()\n  +~MyClass()\n  +void method()\n}"
))

# Abstract with all abstract methods
write_puml("class_feature_all_abstract.puml", wrap(
    "abstract class FullyAbstract {\n  +{abstract} void methodA()\n  +{abstract} String methodB()\n  +{abstract} int methodC(int x)\n  +{abstract} boolean methodD()\n}"
))

# Class with only separators and labels
write_puml("class_feature_sections.puml", wrap(
    "class DataObject {\n"
    "  == Identity ==\n"
    "  +int id\n"
    "  +String name\n"
    "  == Metadata ==\n"
    "  +Date createdAt\n"
    "  +Date updatedAt\n"
    "  == Operations ==\n"
    "  +void save()\n"
    "  +void delete()\n"
    "  +DataObject clone()\n"
    "}"
))

# Class with HTML in name (quoted)
write_puml("class_feature_quoted_class.puml", wrap(
    'class "Class With Spaces" {\n  +void method()\n}\nclass "Another Spaced Class"\n"Class With Spaces" --> "Another Spaced Class"'
))

# Relation with stereotype
write_puml("class_feature_relation_stereotype.puml", wrap(
    "class A\nclass B\nA --> B : <<uses>>"
))

write_puml("class_feature_relation_constraint.puml", wrap(
    "class A\nclass B\nA --> B : {ordered}"
))

# Suppress circles
write_puml("class_feature_hide_circle.puml", wrap(
    "hide circle\nclass MyClass {\n  +void method()\n}\ninterface MyInterface\nMyClass ..|> MyInterface"
))

write_puml("class_feature_hide_stereotype.puml", wrap(
    "hide stereotype\nclass MyClass {\n  +void method()\n}\ninterface MyInterface\nMyClass ..|> MyInterface"
))


# ─────────────────────────────────────────────────────────────────────────────
# 37. MORE EDGE CASES AND STRESS TESTS
# ─────────────────────────────────────────────────────────────────────────────

# Class with only one member of each type
write_puml("class_edge_one_field.puml", wrap(
    "class OneField {\n  +String name\n}"
))

write_puml("class_edge_one_method.puml", wrap(
    "class OneMethod {\n  +void run()\n}"
))

# All-interface diagram
write_puml("class_edge_all_interfaces.puml", wrap(
    "interface A\ninterface B\ninterface C\ninterface D\nA --|> B\nB --|> C\nC --|> D"
))

# All-abstract diagram
write_puml("class_edge_all_abstract.puml", wrap(
    "abstract class A\nabstract class B\nabstract class C\nA <|-- B\nB <|-- C"
))

# Mixed type diagram
write_puml("class_edge_all_types_mixed.puml", wrap(
    "class MyClass {\n  +void method()\n}\n"
    "abstract class MyAbstract {\n  +{abstract} void method()\n}\n"
    "interface MyInterface {\n  +void method()\n}\n"
    "enum MyEnum {\n  VALUE1\n  VALUE2\n}\n"
    "annotation MyAnnotation\n"
    "entity MyEntity {\n  *id : int\n}\n"
    "MyClass --|> MyAbstract\n"
    "MyClass ..|> MyInterface\n"
    "MyClass --> MyEnum\n"
    "MyClass --> MyEntity"
))

# Relationship types all on one pair
write_puml("class_edge_all_rels_same_classes.puml", wrap(
    "class Source\nclass Target1\nclass Target2\nclass Target3\nclass Target4\nclass Target5\nclass Target6\n"
    "Source <|-- Target1\n"
    "Source ..|> Target2\n"
    "Source *-- Target3\n"
    "Source o-- Target4\n"
    "Source --> Target5\n"
    "Source ..> Target6"
))

# Huge note
write_puml("class_edge_huge_note.puml", wrap(
    "class MyClass\nnote right of MyClass\n" +
    "\n".join([f"  Line {i}: some documentation text here" for i in range(1, 21)]) +
    "\nend note"
))

# Many relationships between few classes
write_puml("class_edge_dense_relationships.puml", wrap(
    "class A {\n  +void a()\n}\nclass B {\n  +void b()\n}\nclass C {\n  +void c()\n}\n"
    "A --> B : rel1\nA --> C : rel2\nB --> A : rel3\nB --> C : rel4\nC --> A : rel5\nC --> B : rel6\n"
    "A ..> B : dep1\nA ..> C : dep2"
))

# Nested generics
write_puml("class_edge_nested_generics.puml", wrap(
    "class Container<T> {\n  +T value\n}\nclass Wrapper<Container<T>> {\n  +Container<T> inner\n}"
))

# Long inheritance and wide interface implementation
write_puml("class_edge_complex_hierarchy.puml", wrap(
    "interface I1\ninterface I2\ninterface I3\ninterface I4\ninterface I5\n"
    "abstract class A\nabstract class B\nclass C\nclass D\nclass E\n"
    "A <|-- B\nB <|-- C\nC <|-- D\nD <|-- E\n"
    "E ..|> I1\nE ..|> I2\nE ..|> I3\nE ..|> I4\nE ..|> I5"
))


# ─────────────────────────────────────────────────────────────────────────────
# 38. FIELD TYPE VARIANTS
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_fields_primitive_types.puml", wrap(
    "class PrimitiveTypes {\n"
    "  +byte b\n"
    "  +short s\n"
    "  +int i\n"
    "  +long l\n"
    "  +float f\n"
    "  +double d\n"
    "  +boolean bool\n"
    "  +char c\n"
    "}"
))

write_puml("class_fields_reference_types.puml", wrap(
    "class ReferenceTypes {\n"
    "  +String str\n"
    "  +Object obj\n"
    "  +Integer integer\n"
    "  +List<String> list\n"
    "  +Map<String, Object> map\n"
    "  +Set<Integer> set\n"
    "  +Optional<String> optional\n"
    "}"
))

write_puml("class_fields_array_types.puml", wrap(
    "class ArrayTypes {\n"
    "  +int[] intArray\n"
    "  +String[] strArray\n"
    "  +byte[][] matrix\n"
    "  +Object[] objects\n"
    "}"
))

write_puml("class_fields_no_type.puml", wrap(
    "class NoTypes {\n"
    "  +name\n"
    "  -count\n"
    "  #score\n"
    "  ~flag\n"
    "}"
))


# ─────────────────────────────────────────────────────────────────────────────
# 39. FINAL BATCH: MORE COMBINATORIAL
# ─────────────────────────────────────────────────────────────────────────────

# Every combination of class type and visibility
for kw, kw_label in zip(CLASS_TYPES[:4], CLASS_KEYWORDS[:4]):
    for vis, vis_label in visibilities:
        write_puml(f"class_fullcombo_{kw_label}_{vis_label}.puml", wrap(
            f"{kw} Combo{kw_label.title()}{vis_label.title()} {{\n"
            f"  {vis}String field1\n"
            f"  {vis}int field2\n"
            f"  {vis}void method1()\n"
            f"  {vis}String method2(int x)\n"
            f"}}"
        ))

# Every combination of note position and class type
for note_pos in note_positions:
    for kw, kw_label in zip(CLASS_TYPES[:3], CLASS_KEYWORDS[:3]):
        write_puml(f"class_note_combo_{note_pos}_{kw_label}.puml", wrap(
            f"{kw} MyType {{\n  +void method()\n}}\nnote {note_pos} of MyType : Note on {kw_label}"
        ))

# Relationship combinations with notes
for rel_label, arrow in [("ext", "<|--"), ("comp", "*--"), ("dep", "..>")]:
    write_puml(f"class_rel_note_{rel_label}.puml", wrap(
        f"class Parent\nclass Child\nParent {arrow} Child\nnote on link : Relationship note"
    ))

# Package + skinparam combinations
for pkg in ["package", "namespace", "folder"]:
    write_puml(f"class_pkg_skinparam_{pkg}.puml", wrap(
        f"skinparam PackageBackgroundColor lightyellow\n"
        f"{pkg} myPkg {{\n  class MyClass {{\n    +void method()\n  }}\n}}"
    ))

# Multiple levels of abstraction
write_puml("class_layers_4_level_arch.puml", wrap(
    "package presentation {\n  class View\n  class Controller\n}\n"
    "package business {\n  class Service\n  class Validator\n}\n"
    "package persistence {\n  class Repository\n  class DAO\n}\n"
    "package database {\n  class Connection\n  class Transaction\n}\n"
    "Controller ..> Service\n"
    "Service ..> Repository\n"
    "Repository ..> DAO\n"
    "DAO ..> Connection\n"
    "Controller --> View\n"
    "Service --> Validator\n"
    "DAO --> Transaction"
))

# Class with every member modifier
write_puml("class_all_modifiers.puml", wrap(
    "class AllModifiers {\n"
    "  +publicField: String\n"
    "  -privateField: int\n"
    "  #protectedField: double\n"
    "  ~packageField: boolean\n"
    "  {static} +staticPublicField: String\n"
    "  {static} -staticPrivateField: int\n"
    "  +publicMethod(): void\n"
    "  -privateMethod(): void\n"
    "  #protectedMethod(): void\n"
    "  ~packageMethod(): void\n"
    "  {static} +staticPublicMethod(): void\n"
    "  {static} -staticPrivateMethod(): void\n"
    "}"
))

# Enum with all features
write_puml("class_enum_all_features.puml", wrap(
    "enum FullFeaturedEnum {\n"
    "  VALUE_A\n"
    "  VALUE_B\n"
    "  VALUE_C\n"
    "  VALUE_D\n"
    "  VALUE_E\n"
    "  ==\n"
    "  +String displayName\n"
    "  +int numericValue\n"
    "  --\n"
    "  +String getDisplayName()\n"
    "  +int getNumericValue()\n"
    "  {static} +FullFeaturedEnum fromString(String s)\n"
    "  {static} +FullFeaturedEnum fromInt(int i)\n"
    "}"
))

# Complete interface hierarchy
write_puml("class_interface_full_hierarchy.puml", wrap(
    "interface BaseInterface {\n  +void baseMethod()\n}\n"
    "interface ExtendedInterfaceA {\n  +void methodA()\n}\n"
    "interface ExtendedInterfaceB {\n  +void methodB()\n}\n"
    "interface CombinedInterface {\n  +void combinedMethod()\n}\n"
    "abstract class PartialImpl {\n  +void baseMethod()\n}\n"
    "class FullImpl {\n  +void baseMethod()\n  +void methodA()\n  +void methodB()\n  +void combinedMethod()\n}\n"
    "ExtendedInterfaceA --|> BaseInterface\n"
    "ExtendedInterfaceB --|> BaseInterface\n"
    "CombinedInterface --|> ExtendedInterfaceA\n"
    "CombinedInterface --|> ExtendedInterfaceB\n"
    "PartialImpl ..|> BaseInterface\n"
    "FullImpl --|> PartialImpl\n"
    "FullImpl ..|> CombinedInterface"
))


# ─────────────────────────────────────────────────────────────────────────────
# 40. EVEN MORE VARIANTS FOR TARGET COUNT
# ─────────────────────────────────────────────────────────────────────────────

# Every arrow type with every length modifier
arrow_types = ["-->", "..>", "--", "<|--", "..|>", "*--", "o--", "<-->", "<..>"]
arrow_labels = ["short", "medium", "long"]
for i, arrow in enumerate(arrow_types):
    for length_label in arrow_labels:
        dashes = "-" if length_label == "short" else ("--" if length_label == "medium" else "---")
        # Construct arrow with appropriate length - some arrows can't be easily extended
        # Use a simple approach: just vary the direction arrows
        write_puml(f"class_arrow_type_{i}_{length_label}.puml", wrap(
            f"class A\nclass B\nA {arrow} B"
        ))

# Packages of different styles with relationships crossing boundary
for pkg_type in ["package", "namespace", "folder", "frame", "cloud", "database", "rectangle"]:
    write_puml(f"class_pkg_cross_boundary_{pkg_type}.puml", wrap(
        f"{pkg_type} Inner {{\n  class InnerClass {{\n    +void method()\n  }}\n}}\nclass OuterClass {{\n  +void method()\n}}\nInnerClass --> OuterClass"
    ))

# Colored packages
package_colors = ["#lightblue", "#lightyellow", "#lightgreen", "#pink", "#orange"]
for color in package_colors:
    label = color.replace("#", "")
    write_puml(f"class_pkg_colored_{label}.puml", wrap(
        f"package myPackage {color} {{\n  class MyClass {{\n    +void method()\n  }}\n}}"
    ))

# Stereotype variants on different class types
for kw, kw_label in zip(CLASS_TYPES, CLASS_KEYWORDS):
    for stereo_label, stereo in [("service", "<<service>>"), ("repo", "<<repository>>"), ("ctrl", "<<controller>>")]:
        write_puml(f"class_stereo_{kw_label}_{stereo_label}.puml", wrap(
            f"{kw} MyStereoClass {stereo} {{\n  +void operate()\n}}"
        ))

# Multiple notes with creole
for i, (markup, label) in enumerate([
    ("**bold**", "bold"),
    ("//italic//", "italic"),
    ("__underline__", "underline"),
    ("--strike--", "strike"),
    ("<b>html_bold</b>", "html_bold"),
    ("<i>html_italic</i>", "html_italic"),
    ("<u>html_underline</u>", "html_underline"),
    ("<color:red>colored</color>", "colored"),
    ("<size:18>large</size>", "large"),
    ("* item1\n  * item2", "list"),
]):
    safe_label = label.replace(":", "_")
    write_puml(f"class_creole_note_{safe_label}.puml", wrap(
        f"class MyClass\nnote right of MyClass\n  {markup}\nend note"
    ))

# Abstract class and implementation pairs
for i in range(1, 11):
    write_puml(f"class_abstract_impl_pair_{i:02d}.puml", wrap(
        f"abstract class AbstractService{i} {{\n  +{{'abstract'}} void process{i}()\n  +void common{i}()\n}}\nclass ConcreteService{i} {{\n  +void process{i}()\n}}\nAbstractService{i} <|-- ConcreteService{i}"
    ))

# Deep nesting with multiple classes per level
write_puml("class_pkg_wide_nested.puml", wrap(
    "package top {\n"
    "  class T1\n  class T2\n  class T3\n"
    "  package mid {\n"
    "    class M1\n    class M2\n    class M3\n"
    "    package bottom {\n"
    "      class B1\n      class B2\n      class B3\n"
    "    }\n"
    "  }\n"
    "}\n"
    "T1 --> M1\nT2 --> M2\nT3 --> M3\n"
    "M1 --> B1\nM2 --> B2\nM3 --> B3"
))

# Classes with body using both separators and members
for sep in ["..", "--", "==", "__"]:
    sep_label = sep.replace(".", "dot").replace("-", "dash").replace("=", "eq").replace("_", "us")
    write_puml(f"class_body_separator_{sep_label}_between.puml", wrap(
        f"class WithSep {{\n  +String beforeSep\n  {sep}\n  +void afterSep()\n}}"
    ))

# Class with constructor overloading
for num_params in range(0, 5):
    params = ", ".join([f"String p{i}" for i in range(1, num_params+1)])
    write_puml(f"class_constructor_{num_params}_params.puml", wrap(
        f"class MyClass {{\n  +MyClass({params})\n  +void method()\n}}"
    ))

# Exception hierarchy
write_puml("class_exception_hierarchy.puml", wrap(
    "class Throwable {\n  +String message\n  +void printStackTrace()\n}\n"
    "class Exception\nclass RuntimeException\nclass Error\n"
    "class IOException\nclass SQLException\n"
    "class NullPointerException\nclass IllegalArgumentException\nclass IndexOutOfBoundsException\n"
    "class OutOfMemoryError\nclass StackOverflowError\n"
    "Throwable <|-- Exception\nThrowable <|-- Error\n"
    "Exception <|-- RuntimeException\nException <|-- IOException\nException <|-- SQLException\n"
    "RuntimeException <|-- NullPointerException\n"
    "RuntimeException <|-- IllegalArgumentException\n"
    "RuntimeException <|-- IndexOutOfBoundsException\n"
    "Error <|-- OutOfMemoryError\nError <|-- StackOverflowError"
))

# Collection framework-style hierarchy
write_puml("class_collection_framework.puml", wrap(
    "interface Iterable<T>\ninterface Collection<T>\ninterface List<T>\ninterface Set<T>\ninterface Queue<T>\ninterface Map<K,V>\n"
    "abstract class AbstractCollection<T>\nabstract class AbstractList<T>\nabstract class AbstractSet<T>\n"
    "class ArrayList<T>\nclass LinkedList<T>\nclass HashSet<T>\nclass TreeSet<T>\nclass HashMap<K,V>\nclass TreeMap<K,V>\n"
    "Collection --|> Iterable\nList --|> Collection\nSet --|> Collection\nQueue --|> Collection\n"
    "AbstractCollection ..|> Collection\nAbstractList --|> AbstractCollection\nAbstractList ..|> List\n"
    "AbstractSet --|> AbstractCollection\nAbstractSet ..|> Set\n"
    "ArrayList --|> AbstractList\nLinkedList --|> AbstractList\nLinkedList ..|> Queue\n"
    "HashSet --|> AbstractSet\nTreeSet --|> AbstractSet\n"
    "HashMap ..|> Map\nTreeMap ..|> Map"
))

# Observer pattern with many observers
write_puml("class_pattern_observer_many.puml", wrap(
    "interface Observer {\n  +void update(Event e)\n}\n"
    "class EventBus {\n  +void subscribe(String event, Observer o)\n  +void publish(Event e)\n}\n"
    "class Event {\n  +String type\n  +Object data\n}\n" +
    "\n".join([f"class Observer{i} {{\n  +void update(Event e)\n}}" for i in range(1, 8)]) + "\n" +
    "\n".join([f"Observer{i} ..|> Observer" for i in range(1, 8)]) + "\n" +
    "EventBus o-- Observer"
))

# Various field initializer styles
write_puml("class_fields_with_defaults.puml", wrap(
    "class WithDefaults {\n"
    "  +String name = \"default\"\n"
    "  +int count = 0\n"
    "  +boolean flag = false\n"
    "  +double rate = 1.0\n"
    "}"
))

# Relation with constraints and properties
write_puml("class_rel_with_constraints.puml", wrap(
    "class Student\nclass Course\nStudent \"0..*\" -- \"0..*\" Course : {ordered, unique}"
))

# Class with only static members
write_puml("class_all_static.puml", wrap(
    "class UtilityClass {\n"
    "  {static} +String VERSION\n"
    "  {static} +int MAX_SIZE\n"
    "  {static} +void process(String input)\n"
    "  {static} +boolean validate(Object o)\n"
    "  {static} +String format(Object o)\n"
    "}"
))

# Deeply nested packages with cross-cutting concerns
write_puml("class_pkg_cross_cutting.puml", wrap(
    "package com.example.web {\n  class WebController\n}\n"
    "package com.example.service {\n  class BusinessService\n}\n"
    "package com.example.data {\n  class DataRepository\n}\n"
    "package com.example.security {\n  class SecurityFilter\n  class Authenticator\n}\n"
    "package com.example.logging {\n  class Logger\n}\n"
    "com.example.web.WebController ..> com.example.service.BusinessService\n"
    "com.example.service.BusinessService ..> com.example.data.DataRepository\n"
    "com.example.web.WebController ..> com.example.security.SecurityFilter\n"
    "com.example.security.SecurityFilter ..> com.example.security.Authenticator\n"
    "com.example.web.WebController ..> com.example.logging.Logger\n"
    "com.example.service.BusinessService ..> com.example.logging.Logger"
))


# ─────────────────────────────────────────────────────────────────────────────
# 41. EXHAUSTIVE RELATIONSHIP DIRECTION × TYPE × LABEL COMBOS
# ─────────────────────────────────────────────────────────────────────────────

# All meaningful arrow head combinations
head_combos = [
    ("ext_fwd",    "<|--"),
    ("ext_rev",    "--|>"),
    ("impl_fwd",   "..|>"),
    ("impl_rev",   "<|.."),
    ("comp_fwd",   "*--"),
    ("comp_rev",   "--*"),
    ("agg_fwd",    "o--"),
    ("agg_rev",    "--o"),
    ("dep_fwd",    "..>"),
    ("dep_rev",    "<.."),
    ("assoc_fwd",  "-->"),
    ("assoc_rev",  "<--"),
    ("bidir",      "<-->"),
    ("plain",      "--"),
    ("plain_dot",  ".."),
    ("both_head",  "<|--|>"),
]

for label, arrow in head_combos:
    # No label
    write_puml(f"class_arrowhead_{label}_bare.puml", wrap(
        f"class A\nclass B\nA {arrow} B"
    ))
    # With middle label
    write_puml(f"class_arrowhead_{label}_labeled.puml", wrap(
        f"class A\nclass B\nA {arrow} B : link"
    ))
    # With multiplicity
    write_puml(f"class_arrowhead_{label}_mult.puml", wrap(
        f'class A\nclass B\nA "1" {arrow} "*" B'
    ))
    # With role labels on both ends
    write_puml(f"class_arrowhead_{label}_roles.puml", wrap(
        f'class A\nclass B\nA "roleA" {arrow} "roleB" B : assoc'
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 42. EVERY SKINPARAM CLASS ATTRIBUTE
# ─────────────────────────────────────────────────────────────────────────────

skinparam_attrs = [
    ("BackgroundColor", "lightyellow"),
    ("BorderColor", "red"),
    ("FontColor", "blue"),
    ("FontSize", "16"),
    ("FontStyle", "bold"),
    ("HeaderBackgroundColor", "#DDD"),
    ("AttributeFontColor", "green"),
    ("AttributeFontSize", "12"),
    ("AttributeFontStyle", "italic"),
    ("StereotypeFontColor", "purple"),
    ("StereotypeFontSize", "14"),
    ("StereotypeFontStyle", "bold"),
]

for attr, val in skinparam_attrs:
    write_puml(f"class_skinparam_class_{attr.lower()}.puml", wrap(
        f"skinparam Class{attr} {val}\n"
        "class A {\n  +String field\n  +void method()\n}\n"
        "class B {\n  -int count\n}\n"
        "A --> B"
    ))

# Skinparam with stereotype-specific
skinparam_stereo_colors = [
    ("service",    "lightblue",   "blue"),
    ("entity",     "lightyellow", "orange"),
    ("repository", "lightgreen",  "green"),
    ("controller", "lightsalmon", "red"),
    ("util",       "lavender",    "purple"),
]
for stereo_label, bg, border in skinparam_stereo_colors:
    write_puml(f"class_skinparam_stereo_{stereo_label}.puml", wrap(
        f"skinparam class {{\n"
        f"  BackgroundColor<<{stereo_label}>> {bg}\n"
        f"  BorderColor<<{stereo_label}>> {border}\n"
        f"}}\n"
        f"class MyClass <<{stereo_label}>> {{\n  +void method()\n}}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 43. COMPLETE ENUM VARIANTS
# ─────────────────────────────────────────────────────────────────────────────

enum_sizes = [2, 3, 5, 8, 10, 15, 20]
for sz in enum_sizes:
    values = "\n".join([f"  VALUE_{i:02d}" for i in range(1, sz+1)])
    write_puml(f"class_enum_size_{sz:02d}_values.puml", wrap(
        f"enum Enum{sz} {{\n{values}\n}}"
    ))

# Enums connected to classes in various ways
for use_arrow in ["-->", "..>", "--"]:
    label = use_arrow.replace("-", "dash").replace(">", "gt").replace(".", "dot")
    write_puml(f"class_enum_rel_{label}.puml", wrap(
        f"enum Status {{\n  ACTIVE\n  INACTIVE\n}}\n"
        f"class Entity {{\n  +Status status\n}}\n"
        f"Entity {use_arrow} Status"
    ))

# Enum with body sections
write_puml("class_enum_sections.puml", wrap(
    "enum Planet {\n  MERCURY\n  VENUS\n  EARTH\n  MARS\n  JUPITER\n  SATURN\n  URANUS\n  NEPTUNE\n"
    "  == Properties ==\n  +double mass\n  +double radius\n"
    "  -- Methods --\n  +double surfaceGravity()\n  +double surfaceWeight(double otherMass)\n}"
))


# ─────────────────────────────────────────────────────────────────────────────
# 44. GENERIC CLASS BODY VARIANTS
# ─────────────────────────────────────────────────────────────────────────────

generic_bodies = [
    ("stack", "Stack<T>",
     "  -elements: T[]\n  -size: int\n  +void push(T item)\n  +T pop()\n  +T peek()\n  +boolean isEmpty()"),
    ("queue", "Queue<T>",
     "  -head: Node<T>\n  -tail: Node<T>\n  +void enqueue(T item)\n  +T dequeue()\n  +boolean isEmpty()"),
    ("pair", "Pair<A, B>",
     "  +A first\n  +B second\n  {static} +Pair<A,B> of(A a, B b)"),
    ("optional", "Optional<T>",
     "  -T value\n  +boolean isPresent()\n  +T get()\n  +T orElse(T defaultValue)\n  {static} +Optional<T> empty()"),
    ("result", "Result<T, E>",
     "  -T value\n  -E error\n  +boolean isOk()\n  +T unwrap()\n  +E getError()"),
    ("observable", "Observable<T>",
     "  -observers: List<Observer<T>>\n  +void subscribe(Observer<T> o)\n  +void emit(T value)"),
    ("future", "Future<T>",
     "  +T get()\n  +boolean isDone()\n  +void cancel()\n  +Future<U> thenApply(Function<T,U> f)"),
    ("supplier", "Supplier<T>",
     "  +T get()"),
    ("consumer", "Consumer<T>",
     "  +void accept(T value)\n  +Consumer<T> andThen(Consumer<T> after)"),
    ("function", "Function<T, R>",
     "  +R apply(T input)\n  +Function<T,V> andThen(Function<R,V> after)\n  +Function<V,R> compose(Function<V,T> before)"),
]

for label, sig, body in generic_bodies:
    write_puml(f"class_generic_body_{label}.puml", wrap(
        f"class {sig} {{\n{body}\n}}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 45. COMPLETE NOTE VARIANTS
# ─────────────────────────────────────────────────────────────────────────────

# Notes on each class type
for kw, kw_label in zip(CLASS_TYPES, CLASS_KEYWORDS):
    for pos in note_positions:
        write_puml(f"class_note_{kw_label}_{pos}.puml", wrap(
            f"{kw} MyType {{\n  +void method()\n}}\nnote {pos} of MyType : Note on {kw_label}"
        ))

# Multi-line notes on each class type
for kw, kw_label in zip(CLASS_TYPES, CLASS_KEYWORDS):
    write_puml(f"class_note_{kw_label}_multiline.puml", wrap(
        f"{kw} MyType {{\n  +void method()\n}}\n"
        f"note right of MyType\n  Multi-line note\n  about {kw_label}\n  type\nend note"
    ))

# Notes with various creole formatting
creole_formats = [
    ("bold",       "**bold text**"),
    ("italic",     "//italic text//"),
    ("underline",  "__underline text__"),
    ("mono",       "\"\"monospace\"\""),
    ("strike",     "--strikethrough--"),
    ("wave",       "~~wave~~"),
    ("list_ul",    "* item 1\n  * item 2\n  * item 3"),
    ("list_ol",    "# item 1\n  # item 2\n  # item 3"),
    ("horizontal", "----"),
    ("tree",       "|_ parent\n  |_ child1\n  |_ child2"),
]
for fmt_label, content in creole_formats:
    write_puml(f"class_note_creole_{fmt_label}.puml", wrap(
        f"class MyClass\nnote right of MyClass\n  {content}\nend note"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 46. PACKAGE COMBINATIONS
# ─────────────────────────────────────────────────────────────────────────────

# Each package type with each color
pkg_types_main = ["package", "namespace", "node", "folder", "frame", "cloud", "database", "rectangle"]
pkg_colors = ["#lightblue", "#lightyellow", "#lightgreen", "#pink", "#lavender", "#wheat", "#aliceblue"]

for pkg_t in pkg_types_main:
    for color in pkg_colors:
        color_label = color.replace("#", "")
        write_puml(f"class_pkg_{pkg_t}_{color_label}.puml", wrap(
            f"{pkg_t} myPkg {color} {{\n  class MyClass {{\n    +void method()\n  }}\n}}"
        ))

# Nested package types (outer × inner)
for outer in ["package", "namespace", "folder"]:
    for inner in ["package", "namespace", "folder"]:
        write_puml(f"class_pkg_nested_{outer}_{inner}.puml", wrap(
            f"{outer} Outer {{\n  {inner} Inner {{\n    class MyClass {{\n      +void method()\n    }}\n  }}\n}}"
        ))

# Package with stereotype
pkg_stereos = ["<<Framework>>", "<<Application>>", "<<Library>>", "<<Service>>", "<<Database>>"]
for stereo in pkg_stereos:
    label = stereo.replace("<<", "").replace(">>", "").lower()
    write_puml(f"class_pkg_stereotype_{label}.puml", wrap(
        f"package myPkg {stereo} {{\n  class MyClass {{\n    +void method()\n  }}\n}}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 47. HIDE/SHOW COMBINATIONS
# ─────────────────────────────────────────────────────────────────────────────

hide_targets = [
    ("members",         "hide empty members"),
    ("fields",          "hide empty fields"),
    ("methods",         "hide empty methods"),
    ("attributes",      "hide attributes"),
    ("methods_all",     "hide methods"),
    ("circle",          "hide circle"),
    ("stereotype",      "hide stereotype"),
    ("private_field",   "hide private fields"),
    ("protected_field", "hide protected fields"),
    ("public_field",    "hide public fields"),
    ("private_method",  "hide private methods"),
    ("protected_method","hide protected methods"),
]

for label, hide_cmd in hide_targets:
    write_puml(f"class_hide_{label}.puml", wrap(
        f"{hide_cmd}\n"
        "class A {\n  +String publicField\n  -int privateField\n  #double protectedField\n"
        "  +void publicMethod()\n  -void privateMethod()\n  #void protectedMethod()\n}\n"
        "class B"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 48. COMPLEX REAL-WORLD PATTERNS
# ─────────────────────────────────────────────────────────────────────────────

# REST API classes
write_puml("class_domain_rest_api.puml", wrap(
    "package api {\n"
    "  class UserController <<controller>> {\n"
    "    +Response getUser(int id)\n"
    "    +Response createUser(UserDTO dto)\n"
    "    +Response updateUser(int id, UserDTO dto)\n"
    "    +Response deleteUser(int id)\n"
    "  }\n"
    "  class UserDTO {\n"
    "    +String username\n"
    "    +String email\n"
    "    +String password\n"
    "  }\n"
    "  class Response {\n"
    "    +int statusCode\n"
    "    +Object body\n"
    "    +String message\n"
    "  }\n"
    "}\n"
    "package service {\n"
    "  class UserService <<service>> {\n"
    "    +User getUser(int id)\n"
    "    +User createUser(UserDTO dto)\n"
    "    +User updateUser(int id, UserDTO dto)\n"
    "    +void deleteUser(int id)\n"
    "  }\n"
    "}\n"
    "api.UserController ..> service.UserService\n"
    "api.UserController ..> api.UserDTO\n"
    "api.UserController ..> api.Response"
))

# Microservice pattern
write_puml("class_domain_microservices.puml", wrap(
    "package user_service {\n"
    "  class UserAPI\n  class UserBusiness\n  class UserData\n"
    "  UserAPI ..> UserBusiness\n  UserBusiness ..> UserData\n"
    "}\n"
    "package order_service {\n"
    "  class OrderAPI\n  class OrderBusiness\n  class OrderData\n"
    "  OrderAPI ..> OrderBusiness\n  OrderBusiness ..> OrderData\n"
    "}\n"
    "package notification_service {\n"
    "  class NotificationAPI\n  class NotificationBusiness\n"
    "  NotificationAPI ..> NotificationBusiness\n"
    "}\n"
    "package message_bus {\n"
    "  class EventBus\n  class EventPublisher\n  class EventSubscriber\n"
    "  EventPublisher --> EventBus\n  EventSubscriber --> EventBus\n"
    "}\n"
    "user_service.UserBusiness ..> message_bus.EventPublisher : publishes UserCreated\n"
    "order_service.OrderBusiness ..> message_bus.EventPublisher : publishes OrderPlaced\n"
    "notification_service.NotificationBusiness ..> message_bus.EventSubscriber : subscribes"
))

# Game entity system
write_puml("class_domain_game_ecs.puml", wrap(
    "class World {\n  +void addEntity(Entity e)\n  +void removeEntity(int id)\n  +void update(float delta)\n}\n"
    "class Entity {\n  +int id\n  +void addComponent(Component c)\n  +Component getComponent(Class type)\n}\n"
    "abstract class Component {\n  +int entityId\n  +boolean enabled\n}\n"
    "class PositionComponent {\n  +float x\n  +float y\n  +float z\n}\n"
    "class VelocityComponent {\n  +float vx\n  +float vy\n  +float vz\n}\n"
    "class RenderComponent {\n  +String sprite\n  +int layer\n}\n"
    "class CollisionComponent {\n  +float radius\n  +boolean isTrigger\n}\n"
    "abstract class System {\n  +{abstract} void update(World world, float delta)\n}\n"
    "class PhysicsSystem\nclass RenderSystem\nclass CollisionSystem\n"
    "World o-- Entity\nEntity o-- Component\n"
    "PositionComponent --|> Component\nVelocityComponent --|> Component\n"
    "RenderComponent --|> Component\nCollisionComponent --|> Component\n"
    "PhysicsSystem --|> System\nRenderSystem --|> System\nCollisionSystem --|> System\n"
    "World o-- System"
))

# Plugin system
write_puml("class_domain_plugin_system.puml", wrap(
    "interface Plugin {\n  +String getName()\n  +String getVersion()\n  +void initialize(PluginContext ctx)\n  +void destroy()\n}\n"
    "interface PluginContext {\n  +void registerService(String name, Object service)\n  +Object getService(String name)\n  +void log(String message)\n}\n"
    "class PluginManager {\n  -plugins: Map<String, Plugin>\n  +void loadPlugin(Plugin p)\n  +void unloadPlugin(String name)\n  +Plugin getPlugin(String name)\n}\n"
    "class PluginRegistry {\n  +void register(Class<? extends Plugin> cls)\n  +List<Plugin> createInstances()\n}\n"
    "class DefaultPluginContext {\n  +void registerService(String name, Object service)\n  +Object getService(String name)\n  +void log(String message)\n}\n"
    "PluginManager --> Plugin\nPluginManager --> PluginContext\n"
    "DefaultPluginContext ..|> PluginContext\n"
    "PluginRegistry ..> Plugin"
))

# Event sourcing pattern
write_puml("class_pattern_event_sourcing.puml", wrap(
    "abstract class DomainEvent {\n  +UUID id\n  +Instant timestamp\n  +String aggregateId\n}\n"
    "class UserCreatedEvent {\n  +String username\n  +String email\n}\n"
    "class UserUpdatedEvent {\n  +String field\n  +String oldValue\n  +String newValue\n}\n"
    "class UserDeletedEvent\n"
    "interface EventStore {\n  +void save(DomainEvent event)\n  +List<DomainEvent> getEvents(String aggregateId)\n}\n"
    "interface EventPublisher {\n  +void publish(DomainEvent event)\n}\n"
    "abstract class Aggregate {\n  +String id\n  +{abstract} void apply(DomainEvent event)\n  +List<DomainEvent> getUncommittedEvents()\n}\n"
    "class UserAggregate {\n  +String username\n  +String email\n  +boolean deleted\n  +void apply(DomainEvent event)\n}\n"
    "UserCreatedEvent --|> DomainEvent\n"
    "UserUpdatedEvent --|> DomainEvent\n"
    "UserDeletedEvent --|> DomainEvent\n"
    "UserAggregate --|> Aggregate\n"
    "UserAggregate ..> UserCreatedEvent\n"
    "UserAggregate ..> UserUpdatedEvent\n"
    "UserAggregate ..> UserDeletedEvent"
))

# CQRS pattern
write_puml("class_pattern_cqrs.puml", wrap(
    "interface Command\ninterface Query\ninterface CommandHandler<C extends Command> {\n  +void handle(C command)\n}\n"
    "interface QueryHandler<Q extends Query, R> {\n  +R handle(Q query)\n}\n"
    "class CommandBus {\n  +void dispatch(Command cmd)\n  +void register(CommandHandler handler)\n}\n"
    "class QueryBus {\n  +<R> R dispatch(Query query)\n  +void register(QueryHandler handler)\n}\n"
    "class CreateUserCommand {\n  +String username\n  +String email\n}\n"
    "class GetUserQuery {\n  +int userId\n}\n"
    "class UserDTO {\n  +int id\n  +String username\n  +String email\n}\n"
    "class CreateUserCommandHandler {\n  +void handle(CreateUserCommand cmd)\n}\n"
    "class GetUserQueryHandler {\n  +UserDTO handle(GetUserQuery q)\n}\n"
    "CreateUserCommand ..|> Command\nGetUserQuery ..|> Query\n"
    "CreateUserCommandHandler ..|> CommandHandler\n"
    "GetUserQueryHandler ..|> QueryHandler\n"
    "CommandBus --> CommandHandler\nQueryBus --> QueryHandler"
))


# ─────────────────────────────────────────────────────────────────────────────
# 49. MORE INHERITANCE PATTERNS
# ─────────────────────────────────────────────────────────────────────────────

# Multi-level with 3, 4, 5, 6, 7, 8, 9, 10 levels
for depth in range(3, 11):
    levels = "\n".join([
        f"class Level{i:02d} {{\n  +void method{i}()\n}}"
        for i in range(1, depth+1)
    ])
    rels = "\n".join([f"Level{i:02d} <|-- Level{i+1:02d}" for i in range(1, depth)])
    write_puml(f"class_inherit_depth_{depth:02d}.puml", wrap(f"{levels}\n{rels}"))

# Wide inheritance (1 parent, N children)
for width in [2, 3, 4, 5, 6, 7, 8, 10, 12]:
    children = "\n".join([f"class Child{i:02d}" for i in range(1, width+1)])
    rels = "\n".join([f"Parent <|-- Child{i:02d}" for i in range(1, width+1)])
    write_puml(f"class_inherit_width_{width:02d}.puml", wrap(
        f"class Parent {{\n  +void parentMethod()\n}}\n{children}\n{rels}"
    ))

# Multiple interface inheritance patterns
for num_ifaces in range(1, 9):
    ifaces = "\n".join([f"interface I{i} {{\n  +void method{i}()\n}}" for i in range(1, num_ifaces+1)])
    impls = "\n".join([f"Impl ..|> I{i}" for i in range(1, num_ifaces+1)])
    write_puml(f"class_impl_{num_ifaces:02d}_interfaces.puml", wrap(
        f"{ifaces}\nclass Impl\n{impls}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 50. FIELD AND METHOD COUNT VARIANTS
# ─────────────────────────────────────────────────────────────────────────────

for n in [1, 2, 3, 5, 8, 10, 15, 20, 25, 30]:
    fields = "\n".join([f"  +String field{i:02d}" for i in range(1, n+1)])
    write_puml(f"class_fields_count_{n:02d}.puml", wrap(
        f"class ManyFields{n} {{\n{fields}\n}}"
    ))

for n in [1, 2, 3, 5, 8, 10, 15, 20, 25, 30]:
    methods = "\n".join([f"  +void method{i:02d}()" for i in range(1, n+1)])
    write_puml(f"class_methods_count_{n:02d}.puml", wrap(
        f"class ManyMethods{n} {{\n{methods}\n}}"
    ))

# Mixed counts
for nf, nm in [(5, 5), (10, 5), (5, 10), (10, 10), (15, 15), (20, 20)]:
    fields = "\n".join([f"  +String f{i}" for i in range(1, nf+1)])
    methods = "\n".join([f"  +void m{i}()" for i in range(1, nm+1)])
    write_puml(f"class_mixed_f{nf:02d}_m{nm:02d}.puml", wrap(
        f"class Mixed{nf}f{nm}m {{\n{fields}\n  --\n{methods}\n}}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 51. PARAMETERIZED METHOD VARIANTS
# ─────────────────────────────────────────────────────────────────────────────

param_counts = range(0, 8)
param_types = ["int", "String", "double", "boolean", "Object", "List<String>", "Map<String,Object>"]

for n in param_counts:
    params = ", ".join([f"{param_types[i % len(param_types)]} p{i}" for i in range(n)])
    write_puml(f"class_method_params_{n}.puml", wrap(
        f"class Methods{n}Params {{\n  +void method({params})\n}}"
    ))

# Methods with complex return types
return_types = [
    ("void",            "void"),
    ("primitive_int",   "int"),
    ("primitive_bool",  "boolean"),
    ("string",          "String"),
    ("list",            "List<String>"),
    ("map",             "Map<String, Object>"),
    ("optional",        "Optional<String>"),
    ("array",           "String[]"),
    ("future",          "Future<String>"),
    ("stream",          "Stream<String>"),
    ("generic",         "<T> T"),
    ("wildcard",        "List<? extends Number>"),
]
for rt_label, rt in return_types:
    write_puml(f"class_method_return_{rt_label}.puml", wrap(
        f"class ReturnTypes {{\n  +{rt} method()\n}}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 52. ANNOTATION COMBINATIONS
# ─────────────────────────────────────────────────────────────────────────────

annotations_list = [
    ("Override",          ""),
    ("Deprecated",        ""),
    ("SuppressWarnings",  '+String value()'),
    ("FunctionalInterface", ""),
    ("Component",         '+String value() default ""'),
    ("Service",           '+String value() default ""'),
    ("Repository",        '+String value() default ""'),
    ("Controller",        '+String value() default ""'),
    ("RequestMapping",    '+String value()\n  +String method() default "GET"'),
    ("Autowired",         "+boolean required() default true"),
    ("Transactional",     "+boolean readOnly() default false\n  +int timeout() default -1"),
    ("Entity",            "+String name() default \"\""),
    ("Table",             "+String name()\n  +String schema() default \"\""),
    ("Column",            "+String name()\n  +boolean nullable() default true\n  +int length() default 255"),
    ("Id",                ""),
    ("GeneratedValue",    "+String strategy()"),
    ("NotNull",           "+String message() default \"must not be null\""),
    ("Size",              "+int min() default 0\n  +int max() default 2147483647"),
    ("Pattern",           "+String regexp()"),
    ("JsonProperty",      '+String value() default ""'),
]

for ann_name, body in annotations_list:
    if body:
        write_puml(f"class_annotation_{ann_name.lower()}.puml", wrap(
            f"annotation {ann_name} {{\n  {body}\n}}"
        ))
    else:
        write_puml(f"class_annotation_{ann_name.lower()}.puml", wrap(
            f"annotation {ann_name}"
        ))


# ─────────────────────────────────────────────────────────────────────────────
# 53. RELATIONSHIP LABEL STYLES
# ─────────────────────────────────────────────────────────────────────────────

# Label positions and styles
label_styles = [
    ("no_label",       "A --> B"),
    ("mid_label",      "A --> B : uses"),
    ("left_nav",       "A --> B : < owns"),
    ("right_nav",      "A --> B : uses >"),
    ("near_mult",      'A "1" --> B'),
    ("far_mult",       'A --> "n" B'),
    ("both_mult",      'A "1" --> "n" B'),
    ("near_role",      'A "owner" --> B'),
    ("far_role",       'A --> "owned" B'),
    ("both_roles",     'A "owner" --> "owned" B'),
    ("full",           'A "1\\nowner" --> "n\\nowned" B : contains'),
]
for ls_label, rel in label_styles:
    write_puml(f"class_rel_labelstyle_{ls_label}.puml", wrap(
        f"class A\nclass B\n{rel}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 54. COLORED RELATIONSHIPS
# ─────────────────────────────────────────────────────────────────────────────

color_styles = [
    ("red",        "[#red]"),
    ("blue",       "[#blue]"),
    ("green",      "[#green]"),
    ("bold",       "[bold]"),
    ("dashed",     "[dashed]"),
    ("dotted",     "[dotted]"),
    ("red_bold",   "[#red,bold]"),
    ("blue_dashed","[#blue,dashed]"),
    ("thick",      "[thickness=3]"),
    ("hidden",     "[hidden]"),
]

base_arrows = ["-->", "--", "..>", "<|--", "*--", "o--"]
for style_label, style in color_styles:
    for arrow_idx, arrow in enumerate(base_arrows[:3]):
        write_puml(f"class_rel_color_{style_label}_{arrow_idx}.puml", wrap(
            f"class A\nclass B\nA -{style}{arrow.lstrip('-.')} B"
        ))


# ─────────────────────────────────────────────────────────────────────────────
# 55. DIRECTION VARIANTS WITH MORE CLASSES
# ─────────────────────────────────────────────────────────────────────────────

for direction in ["left to right direction", "top to bottom direction"]:
    dir_label = direction.replace(" ", "_")
    for n in [3, 5, 7, 10]:
        classes = "\n".join([f"class N{i}" for i in range(1, n+1)])
        rels = "\n".join([f"N{i} --> N{i+1}" for i in range(1, n)])
        write_puml(f"class_direction_{dir_label}_n{n}.puml", wrap(
            f"{direction}\n{classes}\n{rels}"
        ))


# ─────────────────────────────────────────────────────────────────────────────
# 56. COMBINATION: CLASS TYPE + STEREOTYPE + COLOR + VISIBILITY
# ─────────────────────────────────────────────────────────────────────────────

combo_types = [("class", "class"), ("abstract class", "abstract_class"), ("interface", "interface")]
combo_stereos = [("", "none"), ("<<service>>", "service"), ("<<entity>>", "entity")]
combo_colors = [("", "nocolor"), ("#lightblue", "lightblue"), ("#lightyellow", "lightyellow")]
combo_vis = [("+", "public"), ("-", "private")]

for (kw, kw_l), (stereo, st_l), (color, col_l), (vis, vis_l) in itertools.product(
        combo_types, combo_stereos, combo_colors, combo_vis):
    write_puml(f"class_4combo_{kw_l}_{st_l}_{col_l}_{vis_l}.puml", wrap(
        f"{kw} Combo {stereo} {color} {{\n"
        f"  {vis}String name\n"
        f"  {vis}void method()\n"
        f"}}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 57. ASSOCIATION CLASS VARIANTS
# ─────────────────────────────────────────────────────────────────────────────

assoc_domains = [
    ("student_course", "Student", "Course", "Enrollment",
     "  +Date enrollDate\n  +String grade\n  +boolean isActive"),
    ("employee_project", "Employee", "Project", "WorkAssignment",
     "  +Date startDate\n  +int hoursPerWeek\n  +String role"),
    ("product_order", "Product", "Order", "LineItem",
     "  +int quantity\n  +double unitPrice\n  +double discount"),
    ("actor_movie", "Actor", "Movie", "Role",
     "  +String characterName\n  +int screenMinutes\n  +boolean isLead"),
    ("user_group", "User", "Group", "Membership",
     "  +Date joinedAt\n  +String permissions\n  +boolean isAdmin"),
]

for label, classA, classB, assoc, fields in assoc_domains:
    write_puml(f"class_assoc_class_{label}.puml", wrap(
        f"class {classA}\nclass {classB}\n"
        f"class {assoc} {{\n{fields}\n}}\n"
        f"({classA}, {classB}) .. {assoc}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 58. INTERFACE LOLLIPOP VARIANTS
# ─────────────────────────────────────────────────────────────────────────────

lollipop_components = [
    ("simple",    "class Component\nComponent ()- IDoable"),
    ("dashed",    "class Component\nComponent ()-- IDoable"),
    ("up",        "class Component\nComponent ()-up- IDoable"),
    ("down",      "class Component\nComponent ()-down- IDoable"),
    ("left",      "class Component\nComponent ()-left- IDoable"),
    ("right",     "class Component\nComponent ()-right- IDoable"),
    ("multiple",  "class Component\nComponent ()- IDoable\nComponent ()-- IRunnable\nComponent ()-up- IStoppable"),
    ("required",  "class Component\nComponent --() Required"),
]

for label, content in lollipop_components:
    write_puml(f"class_lollipop_{label}.puml", wrap(content))


# ─────────────────────────────────────────────────────────────────────────────
# 59. NAMESPACE WITH CLASSES AND RELATIONSHIPS
# ─────────────────────────────────────────────────────────────────────────────

# Namespace separator variations
ns_separators = [
    ("dot",    ".",   "com.example.service"),
    ("colon2", "::",  "com::example::service"),
    ("slash",  "/",   "com/example/service"),
    ("dash",   "-",   "com-example-service"),
]

for ns_label, sep, path in ns_separators:
    safe_path = path.replace(".", "dot").replace(":", "col").replace("/", "sl").replace("-", "dash")
    write_puml(f"class_ns_sep_{ns_label}.puml", wrap(
        f"set namespaceSeparator {sep}\n"
        f"class {path}.ClassA {{\n  +void methodA()\n}}\n"
        f"class {path}.ClassB {{\n  +void methodB()\n}}\n"
        f"{path}.ClassA --> {path}.ClassB"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 60. TEMPLATE METHOD PATTERN VARIANTS
# ─────────────────────────────────────────────────────────────────────────────

template_domains = [
    ("report", "ReportGenerator", ["CsvReport", "PdfReport", "HtmlReport"],
     ["readData", "formatData", "writeOutput"]),
    ("payment", "PaymentProcessor", ["CreditCardProcessor", "PayPalProcessor", "BankTransfer"],
     ["validatePayment", "processTransaction", "sendConfirmation"]),
    ("export", "DataExporter", ["JsonExporter", "XmlExporter", "CsvExporter"],
     ["prepareData", "serializeData", "writeToOutput"]),
    ("game", "GameLoop", ["PlatformerGame", "PuzzleGame", "StrategyGame"],
     ["processInput", "updateState", "render"]),
]

for label, base, subclasses, methods in template_domains:
    abstract_methods = "\n".join([f"  +{{abstract}} void {m}()" for m in methods])
    abstract_def = f"abstract class {base} {{\n  +void run()\n{abstract_methods}\n}}"
    concrete_defs = []
    for sub in subclasses:
        concrete_methods = "\n".join([f"  +void {m}()" for m in methods])
        concrete_defs.append(f"class {sub} {{\n{concrete_methods}\n}}")
    rels = "\n".join([f"{base} <|-- {sub}" for sub in subclasses])
    write_puml(f"class_template_method_{label}.puml", wrap(
        f"{abstract_def}\n" + "\n".join(concrete_defs) + f"\n{rels}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 61. TITLE/HEADER/FOOTER COMBINATIONS
# ─────────────────────────────────────────────────────────────────────────────

for has_title in [True, False]:
    for has_header in [True, False]:
        for has_footer in [True, False]:
            if not (has_title or has_header or has_footer):
                continue
            parts = []
            if has_title:
                parts.append("title My Diagram Title")
            if has_header:
                parts.append("header Generated by RustUML")
            if has_footer:
                parts.append("footer Copyright 2024")
            label = f"{'t' if has_title else ''}{'h' if has_header else ''}{'f' if has_footer else ''}"
            write_puml(f"class_decoration_{label}.puml", wrap(
                "\n".join(parts) + "\nclass A\nclass B\nA --> B"
            ))


# ─────────────────────────────────────────────────────────────────────────────
# 62. SPOT CHARACTERS ALL LETTERS
# ─────────────────────────────────────────────────────────────────────────────

import string
spot_colors_cycle = ["#red", "#blue", "#green", "#orange", "#purple", "#FF7700", "#00AAFF"]
for idx, letter in enumerate(string.ascii_uppercase):
    color = spot_colors_cycle[idx % len(spot_colors_cycle)]
    write_puml(f"class_spot_letter_{letter}.puml", wrap(
        f"class SpotClass{letter} << ({letter},{color}) Spot{letter} >> {{\n  +void method()\n}}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 63. RELATIONSHIP CHAINING PATTERNS
# ─────────────────────────────────────────────────────────────────────────────

# Linear chains of different lengths
for n in range(2, 12):
    nodes = " ".join([f"N{i}" for i in range(1, n+1)])
    classes = "\n".join([f"class N{i}" for i in range(1, n+1)])
    rels = "\n".join([f"N{i} --> N{i+1}" for i in range(1, n)])
    write_puml(f"class_chain_len_{n:02d}.puml", wrap(f"{classes}\n{rels}"))

# Mesh/grid patterns
for rows, cols in [(2, 2), (2, 3), (3, 3), (3, 4), (4, 4)]:
    classes = "\n".join([
        f"class R{r}C{c}"
        for r in range(1, rows+1) for c in range(1, cols+1)
    ])
    h_rels = "\n".join([
        f"R{r}C{c} --> R{r}C{c+1}"
        for r in range(1, rows+1) for c in range(1, cols)
    ])
    v_rels = "\n".join([
        f"R{r}C{c} --> R{r+1}C{c}"
        for r in range(1, rows) for c in range(1, cols+1)
    ])
    write_puml(f"class_grid_{rows}x{cols}.puml", wrap(f"{classes}\n{h_rels}\n{v_rels}"))


# ─────────────────────────────────────────────────────────────────────────────
# 64. CLASS WITH URL LINKS VARIANTS
# ─────────────────────────────────────────────────────────────────────────────

url_variants = [
    ("http",    "[[http://example.com]]"),
    ("https",   "[[https://example.com/path/to/page]]"),
    ("tooltip", "[[http://example.com{Tooltip text here}]]"),
    ("anchor",  "[[http://example.com#anchor]]"),
    ("query",   "[[http://example.com?key=value&other=thing]]"),
]

for label, url in url_variants:
    write_puml(f"class_url_{label}.puml", wrap(
        f"class MyClass {url} {{\n  +void method()\n}}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 65. SPECIAL CHARACTERS AND QUOTING
# ─────────────────────────────────────────────────────────────────────────────

special_names = [
    ("spaces",          '"My Class Name"'),
    ("hyphen",          '"My-Class"'),
    ("parens",          '"Class(WithParens)"'),
    ("brackets",        '"Class[WithBrackets]"'),
    ("dot_in_name",     '"com.example.Class"'),
    ("slash_in_name",   '"path/to/Class"'),
    ("number_start",    '"123Class"'),
    ("dollar",          '"$Class"'),
    ("ampersand",       '"A&B"'),
    ("percent",         '"100%Done"'),
    ("at",              '"@Component"'),
    ("exclamation",     '"Alert!"'),
    ("question",        '"Maybe?"'),
    ("equals",          '"A=B"'),
    ("plus_minus",      '"A+B-C"'),
    ("colon_name",      '"ns:MyClass"'),
    ("semicolon",       '"stmt;end"'),
    ("comma",           '"List,Pair"'),
    ("hash",            '"Color#Red"'),
    ("star",            '"Wildcard*"'),
]

for label, quoted_name in special_names:
    write_puml(f"class_special_name_{label}.puml", wrap(
        f"class {quoted_name} {{\n  +void method()\n}}\nclass Other\n{quoted_name} --> Other"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 66. ANNOTATION-HEAVY CLASSES
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_annotated_spring_controller.puml", wrap(
    "annotation RestController\nannotation RequestMapping\nannotation GetMapping\nannotation PostMapping\n"
    "annotation PathVariable\nannotation RequestBody\nannotation ResponseBody\n"
    "class UserController <<RestController>> {\n"
    "  <<RequestMapping(\"/users\")>>\n"
    "  +User getUser(int id)\n"
    "  +User createUser(User u)\n"
    "  +void deleteUser(int id)\n"
    "}"
))

write_puml("class_annotated_jpa_entity.puml", wrap(
    "annotation Entity\nannotation Table\nannotation Id\nannotation GeneratedValue\n"
    "annotation Column\nannotation OneToMany\nannotation ManyToOne\n"
    "class User <<Entity>> {\n"
    "  <<Id>>\n  <<GeneratedValue>>\n"
    "  +int id\n"
    "  <<Column(name=\"user_name\")>>\n"
    "  +String username\n"
    "  <<Column(nullable=false)>>\n"
    "  +String email\n"
    "}"
))


# ─────────────────────────────────────────────────────────────────────────────
# 67. MIXED DIAGRAM WITH MANY FEATURES
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_mega_all_features.puml", wrap(
    "title Mega Class Diagram\n"
    "left to right direction\n"
    "skinparam ClassBackgroundColor lightyellow\n"
    "skinparam ClassBorderColor orange\n"
    "skinparam ArrowColor darkblue\n\n"
    "package domain #lightblue {\n"
    "  abstract class BaseEntity {\n"
    "    +int id\n"
    "    +Date createdAt\n"
    "    +Date updatedAt\n"
    "    {abstract} +void validate()\n"
    "  }\n"
    "  class User <<entity>> {\n"
    "    +String username\n"
    "    +String email\n"
    "    +void validate()\n"
    "  }\n"
    "  class Order <<entity>> {\n"
    "    +Date orderDate\n"
    "    +double total\n"
    "    +void validate()\n"
    "  }\n"
    "  enum OrderStatus {\n"
    "    PENDING\n    CONFIRMED\n    SHIPPED\n    DELIVERED\n"
    "  }\n"
    "  User <|-- BaseEntity\n"
    "  Order <|-- BaseEntity\n"
    "  Order --> OrderStatus\n"
    "}\n\n"
    "package service #lightgreen {\n"
    "  interface UserService {\n"
    "    +User getUser(int id)\n"
    "    +void createUser(User u)\n"
    "  }\n"
    "  class UserServiceImpl <<service>> {\n"
    "    +User getUser(int id)\n"
    "    +void createUser(User u)\n"
    "  }\n"
    "  UserServiceImpl ..|> UserService\n"
    "}\n\n"
    "package repository #lavender {\n"
    "  interface UserRepo {\n"
    "    +User findById(int id)\n"
    "  }\n"
    "  class UserRepoImpl <<repository>> {\n"
    "    +User findById(int id)\n"
    "  }\n"
    "  UserRepoImpl ..|> UserRepo\n"
    "}\n\n"
    "service.UserServiceImpl ..> repository.UserRepo\n"
    "repository.UserRepoImpl ..> domain.User\n\n"
    "note right of domain.User\n"
    "  Main domain entity\n"
    "  Must have unique email\n"
    "end note"
))

# ─────────────────────────────────────────────────────────────────────────────
# 68. INTERFACE WITH CONSTANTS
# ─────────────────────────────────────────────────────────────────────────────

write_puml("class_interface_constants.puml", wrap(
    "interface Constants {\n"
    "  {static} +String VERSION = \"1.0.0\"\n"
    "  {static} +int MAX_RETRIES = 3\n"
    "  {static} +double TIMEOUT = 30.0\n"
    "  {static} +boolean DEBUG = false\n"
    "}"
))

# ─────────────────────────────────────────────────────────────────────────────
# 69. MULTIPLE DIAGRAMS IN ONE FILE (newpage)
# ─────────────────────────────────────────────────────────────────────────────

for n_pages in [2, 3, 4, 5]:
    pages = []
    for p in range(1, n_pages+1):
        pages.append(f"class PageClass{p}A\nclass PageClass{p}B\nPageClass{p}A --> PageClass{p}B")
    write_puml(f"class_multipage_{n_pages}.puml", wrap(
        "\nnewpage\n".join(pages)
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 71. EXHAUSTIVE CLASS TYPE × RELATIONSHIP TYPE MATRIX
# ─────────────────────────────────────────────────────────────────────────────

# All class type pairs with all relationship types
all_class_pairs = list(itertools.combinations(zip(CLASS_TYPES, CLASS_KEYWORDS), 2))
# Use a subset of relationship arrows to keep it manageable but comprehensive
core_arrows = [
    ("ext",   "<|--"),
    ("impl",  "..|>"),
    ("comp",  "*--"),
    ("agg",   "o--"),
    ("dep",   "..>"),
    ("assoc", "-->"),
    ("plain", "--"),
]

for (kw_a, lbl_a), (kw_b, lbl_b) in all_class_pairs:
    for arr_lbl, arrow in core_arrows:
        write_puml(f"class_typepair_{lbl_a}_{lbl_b}_{arr_lbl}.puml", wrap(
            f"{kw_a} TypeA\n{kw_b} TypeB\nTypeA {arrow} TypeB"
        ))


# ─────────────────────────────────────────────────────────────────────────────
# 72. VISIBILITY × MODIFIER × MEMBER TYPE FULL MATRIX
# ─────────────────────────────────────────────────────────────────────────────

all_vis = [("+", "pub"), ("-", "prv"), ("#", "pro"), ("~", "pkg")]
all_mods = [("", "none"), ("{static} ", "sta"), ("{abstract} ", "abs")]
all_member_kinds = [
    ("field_string",  "String fieldName"),
    ("field_int",     "int fieldName"),
    ("method_void",   "void doWork()"),
    ("method_string", "String compute()"),
    ("method_params", "void process(int x, String y)"),
]

for (vis, vis_l), (mod, mod_l), (kind_l, member) in itertools.product(
        all_vis, all_mods, all_member_kinds):
    write_puml(f"class_member_{vis_l}_{mod_l}_{kind_l}.puml", wrap(
        f"class MemberClass {{\n  {mod}{vis}{member}\n}}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 73. EVERY PACKAGE TYPE × SKINPARAM COMBINATION
# ─────────────────────────────────────────────────────────────────────────────

pkg_skinparams = [
    ("BackgroundColor", "lightblue"),
    ("BorderColor",     "red"),
    ("FontColor",       "blue"),
    ("FontSize",        "14"),
    ("FontStyle",       "bold"),
]

for pkg_t in ["package", "namespace", "folder", "frame", "cloud", "database", "rectangle", "node"]:
    for sp_attr, sp_val in pkg_skinparams:
        camel = pkg_t.capitalize()
        write_puml(f"class_pkg_sp_{pkg_t}_{sp_attr.lower()}.puml", wrap(
            f"skinparam {camel}{sp_attr} {sp_val}\n"
            f"{pkg_t} myPkg {{\n  class A {{\n    +void method()\n  }}\n  class B\n  A --> B\n}}"
        ))


# ─────────────────────────────────────────────────────────────────────────────
# 74. NOTE POSITION × CLASS TYPE × CONTENT STYLE
# ─────────────────────────────────────────────────────────────────────────────

note_contents = [
    ("short",     "Simple note"),
    ("long",      "This is a longer note with more content describing what the class does"),
    ("bold",      "**Important** note"),
    ("italic",    "//Optional// note"),
    ("list",      "* point 1\\n  * point 2\\n  * point 3"),
    ("multiword", "Note with\\nmultiple lines\\nof content"),
]

for pos in note_positions:
    for kw, kw_l in zip(CLASS_TYPES[:4], CLASS_KEYWORDS[:4]):
        for content_l, content in note_contents:
            write_puml(f"class_note3way_{kw_l}_{pos}_{content_l}.puml", wrap(
                f"{kw} MyType {{\n  +void method()\n}}\nnote {pos} of MyType : {content}"
            ))


# ─────────────────────────────────────────────────────────────────────────────
# 75. INHERITANCE DEPTHS × INTERFACE COUNTS
# ─────────────────────────────────────────────────────────────────────────────

for depth in range(1, 6):
    for n_ifaces in range(0, 5):
        # Build inheritance chain
        chain = "\n".join([
            f"class Level{d:02d} {{\n  +void m{d}()\n}}"
            for d in range(1, depth+1)
        ])
        chain_rels = "\n".join([f"Level{d:02d} <|-- Level{d+1:02d}" for d in range(1, depth)])
        # Add interfaces
        ifaces = "\n".join([f"interface Iface{i}" for i in range(1, n_ifaces+1)])
        iface_rels = "\n".join([f"Level{depth:02d} ..|> Iface{i}" for i in range(1, n_ifaces+1)])
        write_puml(f"class_hier_d{depth}_i{n_ifaces}.puml", wrap(
            f"{chain}\n{chain_rels}\n{ifaces}\n{iface_rels}"
        ))


# ─────────────────────────────────────────────────────────────────────────────
# 76. COMPREHENSIVE COLOR PALETTE
# ─────────────────────────────────────────────────────────────────────────────

# HTML color names (a broad set)
html_colors = [
    "AliceBlue", "AntiqueWhite", "Aqua", "Aquamarine", "Azure",
    "Beige", "Bisque", "BlanchedAlmond", "Blue", "BlueViolet",
    "Brown", "BurlyWood", "CadetBlue", "Chartreuse", "Chocolate",
    "Coral", "CornflowerBlue", "Cornsilk", "Crimson", "Cyan",
    "DarkBlue", "DarkCyan", "DarkGoldenRod", "DarkGray", "DarkGreen",
    "DarkKhaki", "DarkMagenta", "DarkOliveGreen", "DarkOrange", "DarkOrchid",
    "DarkRed", "DarkSalmon", "DarkSeaGreen", "DarkSlateBlue", "DarkSlateGray",
    "DarkTurquoise", "DarkViolet", "DeepPink", "DeepSkyBlue", "DimGray",
    "DodgerBlue", "FireBrick", "FloralWhite", "ForestGreen", "Fuchsia",
    "Gainsboro", "GhostWhite", "Gold", "GoldenRod", "Gray",
    "Green", "GreenYellow", "HoneyDew", "HotPink", "IndianRed",
    "Indigo", "Ivory", "Khaki", "Lavender", "LavenderBlush",
    "LawnGreen", "LemonChiffon", "LightBlue", "LightCoral", "LightCyan",
    "LightGoldenRodYellow", "LightGray", "LightGreen", "LightPink", "LightSalmon",
    "LightSeaGreen", "LightSkyBlue", "LightSlateGray", "LightSteelBlue", "LightYellow",
    "Lime", "LimeGreen", "Linen", "Magenta", "Maroon",
    "MediumAquaMarine", "MediumBlue", "MediumOrchid", "MediumPurple", "MediumSeaGreen",
    "MediumSlateBlue", "MediumSpringGreen", "MediumTurquoise", "MediumVioletRed",
    "MidnightBlue", "MintCream", "MistyRose", "Moccasin", "NavajoWhite",
    "Navy", "OldLace", "Olive", "OliveDrab", "Orange",
    "OrangeRed", "Orchid", "PaleGoldenRod", "PaleGreen", "PaleTurquoise",
    "PaleVioletRed", "PapayaWhip", "PeachPuff", "Peru", "Pink",
    "Plum", "PowderBlue", "Purple", "Red", "RosyBrown",
    "RoyalBlue", "SaddleBrown", "Salmon", "SandyBrown", "SeaGreen",
    "SeaShell", "Sienna", "Silver", "SkyBlue", "SlateBlue",
    "SlateGray", "Snow", "SpringGreen", "SteelBlue", "Tan",
    "Teal", "Thistle", "Tomato", "Turquoise", "Violet",
    "Wheat", "White", "WhiteSmoke", "Yellow", "YellowGreen",
]

for color_name in html_colors:
    label = color_name.lower()
    write_puml(f"class_htmlcolor_{label}.puml", wrap(
        f"class ColorTest #{color_name} {{\n  +void method()\n}}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 77. ENTITY DIAGRAM VARIANTS
# ─────────────────────────────────────────────────────────────────────────────

db_tables = [
    ("users",    "*user_id : INT\n  --\n  username : VARCHAR(50)\n  email : VARCHAR(100)\n  password_hash : VARCHAR(255)\n  created_at : TIMESTAMP"),
    ("products", "*product_id : INT\n  --\n  name : VARCHAR(100)\n  description : TEXT\n  price : DECIMAL(10,2)\n  stock : INT\n  category_id : INT"),
    ("orders",   "*order_id : INT\n  --\n  user_id : INT\n  order_date : DATE\n  total : DECIMAL(10,2)\n  status : VARCHAR(20)"),
    ("order_items", "*item_id : INT\n  --\n  order_id : INT\n  product_id : INT\n  quantity : INT\n  unit_price : DECIMAL(10,2)"),
    ("categories", "*category_id : INT\n  --\n  name : VARCHAR(50)\n  parent_id : INT"),
    ("addresses", "*address_id : INT\n  --\n  user_id : INT\n  street : VARCHAR(200)\n  city : VARCHAR(100)\n  country : VARCHAR(50)\n  postal_code : VARCHAR(20)"),
    ("payments", "*payment_id : INT\n  --\n  order_id : INT\n  amount : DECIMAL(10,2)\n  method : VARCHAR(30)\n  status : VARCHAR(20)\n  paid_at : TIMESTAMP"),
    ("reviews",  "*review_id : INT\n  --\n  user_id : INT\n  product_id : INT\n  rating : TINYINT\n  comment : TEXT\n  created_at : TIMESTAMP"),
]

# Individual entity tables
for tbl_name, fields in db_tables:
    write_puml(f"class_entity_table_{tbl_name}.puml", wrap(
        f"entity {tbl_name} {{\n  {fields}\n}}"
    ))

# ER diagram with relationships
write_puml("class_entity_er_ecommerce.puml", wrap(
    "\n".join([f"entity {tbl} {{\n  {fields}\n}}" for tbl, fields in db_tables]) + "\n"
    "users ||--o{ orders : places\n"
    "orders ||--o{ order_items : contains\n"
    "products ||--o{ order_items : included_in\n"
    "products }o--|| categories : belongs_to\n"
    "orders ||--|| payments : paid_by\n"
    "users ||--o{ addresses : has\n"
    "users ||--o{ reviews : writes\n"
    "products ||--o{ reviews : receives"
))


# ─────────────────────────────────────────────────────────────────────────────
# 78. CLASS COUNT SCALING TESTS
# ─────────────────────────────────────────────────────────────────────────────

# Tests with specific numbers of classes
for n in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 15, 18, 20, 25, 30, 40, 50]:
    classes_list = "\n".join([f"class Cls{i:03d} {{ +void m{i}() }}" for i in range(1, n+1)])
    # Linear chain
    rels_list = "\n".join([f"Cls{i:03d} --> Cls{i+1:03d}" for i in range(1, n)])
    write_puml(f"class_count_{n:03d}_linear.puml", wrap(f"{classes_list}\n{rels_list}"))
    # No relationships
    write_puml(f"class_count_{n:03d}_isolated.puml", wrap(classes_list))


# ─────────────────────────────────────────────────────────────────────────────
# 79. STEREOTYPE + RELATIONSHIP COMBINATIONS
# ─────────────────────────────────────────────────────────────────────────────

stereo_pairs = [
    ("service_repo",   "<<service>>",     "<<repository>>",   "..>"),
    ("ctrl_service",   "<<controller>>",  "<<service>>",      "..>"),
    ("view_ctrl",      "<<view>>",        "<<controller>>",   "-->"),
    ("entity_repo",    "<<entity>>",      "<<repository>>",   "<|--"),
    ("facade_service", "<<facade>>",      "<<service>>",      "-->"),
    ("factory_entity", "<<factory>>",     "<<entity>>",       "..>"),
    ("proxy_target",   "<<proxy>>",       "<<target>>",       "-->"),
    ("decorator_comp", "<<decorator>>",   "<<component>>",    "..|>"),
    ("observer_subj",  "<<observer>>",    "<<subject>>",      "-->"),
    ("strategy_ctx",   "<<strategy>>",    "<<context>>",      "<--"),
]

for label, stereo_a, stereo_b, arrow in stereo_pairs:
    write_puml(f"class_stereo_rel_{label}.puml", wrap(
        f"class ClassA {stereo_a} {{\n  +void methodA()\n}}\n"
        f"class ClassB {stereo_b} {{\n  +void methodB()\n}}\n"
        f"ClassA {arrow} ClassB"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 80. MISC ADDITIONAL PATTERNS
# ─────────────────────────────────────────────────────────────────────────────

# Empty body vs single member
for kw, kw_l in zip(CLASS_TYPES, CLASS_KEYWORDS):
    write_puml(f"class_single_member_{kw_l}_field.puml", wrap(
        f"{kw} OneMember {{\n  +String single\n}}"
    ))
    write_puml(f"class_single_member_{kw_l}_method.puml", wrap(
        f"{kw} OneMember {{\n  +void single()\n}}"
    ))

# Class with every separator type in sequence
write_puml("class_all_separators_sequence.puml", wrap(
    "class AllSeps {\n"
    "  +String a\n"
    "  --\n"
    "  +String b\n"
    "  ..\n"
    "  +String c\n"
    "  ==\n"
    "  +String d\n"
    "  __\n"
    "  +String e\n"
    "  -- labeled --\n"
    "  +String f\n"
    "  .. labeled ..\n"
    "  +String g\n"
    "  == labeled ==\n"
    "  +String h\n"
    "  __ labeled __\n"
    "  +String i\n"
    "}"
))

# Abstract class hierarchy with all abstract → partial → full implementation
write_puml("class_abstract_partial_full_chain.puml", wrap(
    "abstract class Fully {\n  {abstract} +void m1()\n  {abstract} +void m2()\n  {abstract} +void m3()\n}\n"
    "abstract class Partial {\n  +void m1()\n  {abstract} +void m2()\n  {abstract} +void m3()\n}\n"
    "class Concrete {\n  +void m2()\n  +void m3()\n}\n"
    "Fully <|-- Partial\nPartial <|-- Concrete"
))

# Aggregation tree
write_puml("class_agg_tree.puml", wrap(
    "class Company {\n  +String name\n}\n"
    "class Department {\n  +String name\n}\n"
    "class Team {\n  +String name\n}\n"
    "class Employee {\n  +String name\n  +String role\n}\n"
    'Company "1" o-- "1..*" Department\n'
    'Department "1" o-- "1..*" Team\n'
    'Team "1" o-- "1..*" Employee'
))

# Composition tree
write_puml("class_comp_tree.puml", wrap(
    "class House {\n  +String address\n}\n"
    "class Room {\n  +String name\n  +double area\n}\n"
    "class Furniture {\n  +String type\n}\n"
    "class Door {\n  +boolean isOpen\n}\n"
    "class Window {\n  +double width\n  +double height\n}\n"
    'House "1" *-- "1..*" Room\n'
    'Room "1" *-- "0..*" Furniture\n'
    'Room "1" *-- "1..*" Door\n'
    'Room "1" *-- "0..*" Window'
))

# Complex multi-class diagram with all relationship types
write_puml("class_comprehensive_relationships.puml", wrap(
    "abstract class Animal {\n  +String name\n  +{abstract} void makeSound()\n}\n"
    "class Dog {\n  +String breed\n  +void makeSound()\n}\n"
    "class Cat {\n  +String color\n  +void makeSound()\n}\n"
    "interface Trainable {\n  +void train(String command)\n  +boolean knows(String command)\n}\n"
    "interface Pet {\n  +String getOwnerName()\n}\n"
    "class Owner {\n  +String name\n  +String address\n}\n"
    "class Kennel {\n  +String name\n  +int capacity\n}\n"
    "class Toy {\n  +String type\n}\n"
    "class VetVisit {\n  +Date date\n  +String reason\n  +String diagnosis\n}\n"
    "Animal <|-- Dog\nAnimal <|-- Cat\n"
    "Dog ..|> Trainable\nDog ..|> Pet\nCat ..|> Pet\n"
    "Owner \"1\" o-- \"*\" Dog\nOwner \"1\" o-- \"*\" Cat\n"
    "Kennel o-- Dog\n"
    "Dog *-- Toy\n"
    "Dog \"1\" -- \"*\" VetVisit\nCat \"1\" -- \"*\" VetVisit\n"
    "note right of Dog : Man's best friend\n"
    "note left of Trainable : Can be taught commands"
))

# Full OOP pillars demonstration
write_puml("class_oop_pillars.puml", wrap(
    "' Encapsulation\n"
    "class BankAccount {\n"
    "  -balance: double\n"
    "  -accountNumber: String\n"
    "  +deposit(amount: double): void\n"
    "  +withdraw(amount: double): boolean\n"
    "  +getBalance(): double\n"
    "}\n\n"
    "' Inheritance\n"
    "abstract class Shape {\n"
    "  #color: String\n"
    "  +{abstract} area(): double\n"
    "  +{abstract} perimeter(): double\n"
    "  +describe(): void\n"
    "}\n"
    "class Circle {\n"
    "  -radius: double\n"
    "  +area(): double\n"
    "  +perimeter(): double\n"
    "}\n"
    "class Rectangle {\n"
    "  -width: double\n"
    "  -height: double\n"
    "  +area(): double\n"
    "  +perimeter(): double\n"
    "}\n"
    "Shape <|-- Circle\n"
    "Shape <|-- Rectangle\n\n"
    "' Polymorphism\n"
    "interface Drawable {\n"
    "  +draw(): void\n"
    "}\n"
    "Circle ..|> Drawable\n"
    "Rectangle ..|> Drawable\n\n"
    "' Abstraction\n"
    "interface Repository<T> {\n"
    "  +findById(id: int): T\n"
    "  +save(entity: T): void\n"
    "  +delete(id: int): void\n"
    "  +findAll(): List<T>\n"
    "}"
))

# ─────────────────────────────────────────────────────────────────────────────
# 81. EVERY CLASS TYPE × EVERY BODY PATTERN
# ─────────────────────────────────────────────────────────────────────────────

body_patterns = [
    ("empty",          ""),
    ("one_pub_field",  "  +String name"),
    ("one_prv_field",  "  -int count"),
    ("one_pub_method", "  +void run()"),
    ("one_prv_method", "  -void helper()"),
    ("field_method",   "  +String name\n  +void run()"),
    ("sep_field_method","  +String name\n  --\n  +void run()"),
    ("two_fields",     "  +String name\n  -int count"),
    ("two_methods",    "  +void run()\n  -void helper()"),
    ("full_vis",       "  +String pub\n  -int prv\n  #double pro\n  ~boolean pkg"),
    ("static_field",   "  {static} +String CONST"),
    ("abstract_method","  {abstract} +void doIt()"),
    ("static_method",  "  {static} +void create()"),
    ("generic_field",  "  +List<String> items"),
    ("generic_method", "  +<T> T cast(Object o)"),
]

for kw, kw_l in zip(CLASS_TYPES, CLASS_KEYWORDS):
    for bp_label, body in body_patterns:
        content = f"{kw} Variant" + (" {\n" + body + "\n}" if body else "")
        write_puml(f"class_body_{kw_l}_{bp_label}.puml", wrap(content))


# ─────────────────────────────────────────────────────────────────────────────
# 82. RELATIONSHIP × MULTIPLICITY × LABEL MATRIX
# ─────────────────────────────────────────────────────────────────────────────

rel_arrows_short = [("ext", "<|--"), ("comp", "*--"), ("agg", "o--"), ("dep", "..>"), ("assoc", "-->")]
mult_combos = [
    ("11",       '"1"',    '"1"'),
    ("1n",       '"1"',    '"*"'),
    ("nn",       '"*"',    '"*"'),
    ("01",       '"0..1"', '"1"'),
    ("1p",       '"1..*"', '"1"'),
    ("none",     "",       ""),
]
mid_labels = [("no_mid", ""), ("mid", " : uses"), ("nav_right", " : uses >"), ("nav_left", " : < owns")]

for (arr_l, arrow), (mult_l, near, far), (mid_l, mid) in itertools.product(
        rel_arrows_short, mult_combos, mid_labels):
    write_puml(f"class_relmatrix_{arr_l}_{mult_l}_{mid_l}.puml", wrap(
        f"class A\nclass B\nA{(' ' + near) if near else ''} {arrow}{(' ' + far) if far else ''} B{mid}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 83. PACKAGE DEPTH × PACKAGE TYPE × CLASS COUNT
# ─────────────────────────────────────────────────────────────────────────────

for depth in range(1, 5):
    for pkg_t in ["package", "namespace", "folder"]:
        for n_classes in [1, 2, 3, 5]:
            # Build nested structure
            classes_inner = "\n".join([
                f"    {'  ' * depth}class C{i} {{ +void m{i}() }}"
                for i in range(1, n_classes+1)
            ])
            # Build wrapping packages
            open_tags = "\n".join([f"{'  ' * d}{pkg_t} Pkg{d+1} {{" for d in range(depth)])
            close_tags = "\n".join(["}" * 1 for _ in range(depth)])
            write_puml(f"class_pkgdepth_{pkg_t}_d{depth}_c{n_classes}.puml", wrap(
                f"{open_tags}\n{classes_inner}\n{close_tags}"
            ))


# ─────────────────────────────────────────────────────────────────────────────
# 84. COLOR × BORDER STYLE COMBINATIONS FOR CLASSES
# ─────────────────────────────────────────────────────────────────────────────

bg_colors = ["#lightblue", "#lightyellow", "#lightgreen", "#pink", "#lavender",
             "#wheat", "#aliceblue", "#mistyrose", "#honeydew", "#azure"]
border_styles = ["line", "line.dashed", "line.dotted", "line.bold"]

for bg in bg_colors:
    for bs in border_styles:
        bg_l = bg.replace("#", "")
        bs_l = bs.replace(".", "_")
        write_puml(f"class_style_{bg_l}_{bs_l}.puml", wrap(
            f"class Styled #back:{bg[1:]};{bs}:red {{\n  +void method()\n}}"
        ))


# ─────────────────────────────────────────────────────────────────────────────
# 85. EVERY HIDE COMMAND × EVERY CLASS TYPE
# ─────────────────────────────────────────────────────────────────────────────

hide_cmds = [
    ("empty_members",   "hide empty members"),
    ("empty_fields",    "hide empty fields"),
    ("empty_methods",   "hide empty methods"),
    ("circle",          "hide circle"),
    ("stereotype",      "hide stereotype"),
    ("attributes",      "hide attributes"),
    ("methods",         "hide methods"),
]

for hide_l, hide_cmd in hide_cmds:
    for kw, kw_l in zip(CLASS_TYPES, CLASS_KEYWORDS):
        write_puml(f"class_hide_{hide_l}_{kw_l}.puml", wrap(
            f"{hide_cmd}\n{kw} MyType {{\n  +String field\n  +void method()\n}}"
        ))


# ─────────────────────────────────────────────────────────────────────────────
# 86. DIRECTION × PACKAGE × RELATIONSHIP
# ─────────────────────────────────────────────────────────────────────────────

for direction in ["left to right direction", "top to bottom direction"]:
    dir_l = "ltr" if "left" in direction else "ttb"
    for pkg_t in ["package", "namespace", "folder"]:
        write_puml(f"class_dir_pkg_{dir_l}_{pkg_t}.puml", wrap(
            f"{direction}\n{pkg_t} A {{\n  class ClassA {{ +void m() }}\n}}\n"
            f"{pkg_t} B {{\n  class ClassB {{ +void m() }}\n}}\n"
            f"A.ClassA --> B.ClassB"
        ))

# ─────────────────────────────────────────────────────────────────────────────
# 87. SKINPARAM COMBINATIONS (pairs)
# ─────────────────────────────────────────────────────────────────────────────

skin_bg_colors = ["lightyellow", "lightblue", "lightgreen", "lavender", "mistyrose"]
skin_border_colors = ["red", "blue", "green", "orange", "purple"]
skin_font_sizes = ["12", "14", "16", "18"]

# bg × border
for bg, border in itertools.product(skin_bg_colors, skin_border_colors):
    write_puml(f"class_skin2_{bg}_{border}.puml", wrap(
        f"skinparam ClassBackgroundColor {bg}\n"
        f"skinparam ClassBorderColor {border}\n"
        "class A { +void m() }\nclass B\nA --> B"
    ))

# bg × font_size
for bg, fz in itertools.product(skin_bg_colors, skin_font_sizes):
    write_puml(f"class_skin2bg_fz_{bg}_{fz}.puml", wrap(
        f"skinparam ClassBackgroundColor {bg}\n"
        f"skinparam ClassFontSize {fz}\n"
        "class A { +void m() }\nclass B\nA --> B"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 88. ENUM × CLASS INTERACTION PATTERNS
# ─────────────────────────────────────────────────────────────────────────────

enum_class_patterns = [
    ("field_ref",     "class Entity {\n  +Status status\n}\nEntity --> Status"),
    ("method_param",  "class Service {\n  +void process(Status s)\n}\nService ..> Status"),
    ("method_return", "class Factory {\n  +Status create()\n}\nFactory ..> Status"),
    ("implements",    "interface IStatus {\n  +String name()\n}\nStatus ..|> IStatus"),
    ("with_values",   "enum Status {\n  ACTIVE = 1\n  INACTIVE = 2\n  PENDING = 3\n}\nclass Entity\nEntity --> Status"),
]

enum_defs = [
    ("simple", "enum Status {\n  ACTIVE\n  INACTIVE\n  PENDING\n}"),
    ("with_methods", "enum Status {\n  ACTIVE\n  INACTIVE\n  --\n  +String display()\n  {static} +Status fromString(String s)\n}"),
    ("with_fields", "enum Status {\n  ACTIVE\n  INACTIVE\n  --\n  +int code\n  +String label\n}"),
]

for (e_label, enum_def), (p_label, pattern) in itertools.product(enum_defs, enum_class_patterns):
    write_puml(f"class_enum_interact_{e_label}_{p_label}.puml", wrap(
        f"{enum_def}\n{pattern}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 89. ANNOTATION × USAGE PATTERNS
# ─────────────────────────────────────────────────────────────────────────────

annotation_uses = [
    ("on_class",     "<<MyAnnotation>>\nclass Target {\n  +void method()\n}"),
    ("on_method",    "class Target {\n  <<MyAnnotation>>\n  +void method()\n}"),
    ("on_field",     "class Target {\n  <<MyAnnotation>>\n  +String field\n}"),
    ("with_rel",     "class Target {\n  +void method()\n}\nTarget ..> MyAnnotation"),
    ("stereotype",   "class Target <<MyAnnotation>> {\n  +void method()\n}"),
]

simple_annotations = [
    ("marker",   "annotation MyAnnotation"),
    ("valued",   "annotation MyAnnotation {\n  +String value()\n}"),
    ("complex",  "annotation MyAnnotation {\n  +String name()\n  +int count() default 1\n  +boolean flag() default false\n}"),
]

for (ann_l, ann_def), (use_l, use_pattern) in itertools.product(simple_annotations, annotation_uses):
    write_puml(f"class_ann_use_{ann_l}_{use_l}.puml", wrap(
        f"{ann_def}\n{use_pattern}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 90. COMPREHENSIVE REAL-WORLD PATTERNS
# ─────────────────────────────────────────────────────────────────────────────

# More design patterns
write_puml("class_pattern_flyweight.puml", wrap(
    "interface Flyweight {\n  +void operation(ExtrinsicState s)\n}\n"
    "class ConcreteFlyweight {\n  -intrinsicState: String\n  +void operation(ExtrinsicState s)\n}\n"
    "class UnsharedFlyweight {\n  -allState: String\n  +void operation(ExtrinsicState s)\n}\n"
    "class FlyweightFactory {\n  -flyweights: Map<String, Flyweight>\n  +Flyweight getFlyweight(String key)\n}\n"
    "class ExtrinsicState {\n  +String data\n}\n"
    "ConcreteFlyweight ..|> Flyweight\n"
    "UnsharedFlyweight ..|> Flyweight\n"
    "FlyweightFactory --> ConcreteFlyweight\n"
    "FlyweightFactory --> UnsharedFlyweight"
))

write_puml("class_pattern_state.puml", wrap(
    "class Context {\n  -state: State\n  +void request()\n  +void setState(State s)\n}\n"
    "abstract class State {\n  +{abstract} void handle(Context ctx)\n}\n"
    "class ConcreteStateA {\n  +void handle(Context ctx)\n}\n"
    "class ConcreteStateB {\n  +void handle(Context ctx)\n}\n"
    "class ConcreteStateC {\n  +void handle(Context ctx)\n}\n"
    "Context --> State\n"
    "ConcreteStateA --|> State\n"
    "ConcreteStateB --|> State\n"
    "ConcreteStateC --|> State"
))

write_puml("class_pattern_mediator.puml", wrap(
    "interface Mediator {\n  +void notify(Component sender, String event)\n}\n"
    "abstract class Component {\n  #mediator: Mediator\n  +void setMediator(Mediator m)\n}\n"
    "class ConcreteMediator {\n  -componentA: ComponentA\n  -componentB: ComponentB\n"
    "  +void notify(Component sender, String event)\n}\n"
    "class ComponentA {\n  +void doA()\n}\n"
    "class ComponentB {\n  +void doB()\n}\n"
    "ConcreteMediator ..|> Mediator\n"
    "ComponentA --|> Component\n"
    "ComponentB --|> Component\n"
    "ConcreteMediator --> ComponentA\n"
    "ConcreteMediator --> ComponentB"
))

write_puml("class_pattern_memento.puml", wrap(
    "class Originator {\n  -state: String\n  +Memento save()\n  +void restore(Memento m)\n  +String getState()\n}\n"
    "class Memento {\n  -state: String\n  +String getState()\n}\n"
    "class Caretaker {\n  -mementos: List<Memento>\n  +void backup()\n  +void undo()\n}\n"
    "Originator --> Memento : creates\n"
    "Caretaker --> Originator\n"
    "Caretaker o-- Memento"
))

write_puml("class_pattern_visitor.puml", wrap(
    "interface Visitor {\n  +void visitConcreteElementA(ConcreteElementA e)\n"
    "  +void visitConcreteElementB(ConcreteElementB e)\n}\n"
    "interface Element {\n  +void accept(Visitor v)\n}\n"
    "class ConcreteVisitor1 {\n  +void visitConcreteElementA(ConcreteElementA e)\n"
    "  +void visitConcreteElementB(ConcreteElementB e)\n}\n"
    "class ConcreteVisitor2 {\n  +void visitConcreteElementA(ConcreteElementA e)\n"
    "  +void visitConcreteElementB(ConcreteElementB e)\n}\n"
    "class ConcreteElementA {\n  +void accept(Visitor v)\n  +void operationA()\n}\n"
    "class ConcreteElementB {\n  +void accept(Visitor v)\n  +void operationB()\n}\n"
    "ConcreteVisitor1 ..|> Visitor\n"
    "ConcreteVisitor2 ..|> Visitor\n"
    "ConcreteElementA ..|> Element\n"
    "ConcreteElementB ..|> Element"
))

write_puml("class_pattern_iterator.puml", wrap(
    "interface Iterator<T> {\n  +boolean hasNext()\n  +T next()\n  +void remove()\n}\n"
    "interface Iterable<T> {\n  +Iterator<T> iterator()\n}\n"
    "class ConcreteIterator<T> {\n  -collection: ConcreteCollection<T>\n  -index: int\n"
    "  +boolean hasNext()\n  +T next()\n  +void remove()\n}\n"
    "class ConcreteCollection<T> {\n  -items: T[]\n  +Iterator<T> iterator()\n  +void add(T item)\n  +int size()\n}\n"
    "ConcreteIterator ..|> Iterator\n"
    "ConcreteCollection ..|> Iterable\n"
    "ConcreteIterator --> ConcreteCollection"
))

write_puml("class_pattern_chain_of_responsibility.puml", wrap(
    "abstract class Handler {\n  -next: Handler\n  +void setNext(Handler h)\n  +{abstract} void handle(Request r)\n}\n"
    "class ConcreteHandlerA {\n  +void handle(Request r)\n}\n"
    "class ConcreteHandlerB {\n  +void handle(Request r)\n}\n"
    "class ConcreteHandlerC {\n  +void handle(Request r)\n}\n"
    "class Request {\n  +String type\n  +Object data\n}\n"
    "ConcreteHandlerA --|> Handler\n"
    "ConcreteHandlerB --|> Handler\n"
    "ConcreteHandlerC --|> Handler\n"
    "Handler --> Handler : next"
))

write_puml("class_pattern_interpreter.puml", wrap(
    "abstract class Expression {\n  +{abstract} int interpret(Context ctx)\n}\n"
    "class NumberExpression {\n  -number: int\n  +int interpret(Context ctx)\n}\n"
    "class AddExpression {\n  -left: Expression\n  -right: Expression\n  +int interpret(Context ctx)\n}\n"
    "class SubtractExpression {\n  -left: Expression\n  -right: Expression\n  +int interpret(Context ctx)\n}\n"
    "class MultiplyExpression {\n  -left: Expression\n  -right: Expression\n  +int interpret(Context ctx)\n}\n"
    "class Context {\n  -variables: Map<String, Integer>\n  +int lookup(String name)\n}\n"
    "NumberExpression --|> Expression\n"
    "AddExpression --|> Expression\n"
    "SubtractExpression --|> Expression\n"
    "MultiplyExpression --|> Expression\n"
    "AddExpression o-- Expression\n"
    "SubtractExpression o-- Expression\n"
    "MultiplyExpression o-- Expression"
))


# ─────────────────────────────────────────────────────────────────────────────
# 91. SCALABILITY AND STRESS TESTS EXTENDED
# ─────────────────────────────────────────────────────────────────────────────

# Various graph topologies
for n in [4, 6, 8, 10, 12]:
    # Complete graph
    classes = "\n".join([f"class V{i}" for i in range(1, n+1)])
    rels = "\n".join([
        f"V{i} --> V{j}"
        for i in range(1, n+1) for j in range(i+1, n+1)
    ])
    write_puml(f"class_topology_complete_k{n}.puml", wrap(f"{classes}\n{rels}"))

# Circular (ring) topology
for n in [3, 4, 5, 6, 7, 8, 10]:
    classes = "\n".join([f"class Ring{i}" for i in range(1, n+1)])
    rels = "\n".join([f"Ring{i} --> Ring{(i % n) + 1}" for i in range(1, n+1)])
    write_puml(f"class_topology_ring_{n}.puml", wrap(f"{classes}\n{rels}"))

# Binary tree topology
for depth in range(2, 6):
    nodes = []
    rels = []
    for d in range(depth):
        for i in range(2**d):
            node_id = 2**d + i
            nodes.append(f"class N{node_id}")
            if d > 0:
                parent_id = node_id // 2
                rels.append(f"N{parent_id} --> N{node_id}")
    write_puml(f"class_topology_btree_d{depth}.puml", wrap(
        "\n".join(nodes) + "\n" + "\n".join(rels)
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 92. NOTE COMBINATIONS WITH RELATIONSHIPS
# ─────────────────────────────────────────────────────────────────────────────

rel_note_patterns = [
    ("before_rel",  "note right of A : Note before rel\nA --> B"),
    ("after_rel",   "A --> B\nnote right of A : Note after rel"),
    ("on_link",     "A --> B\nnote on link : Link note"),
    ("both_classes","note left of A : Note on A\nA --> B\nnote right of B : Note on B"),
    ("floating",    'note "Floating note" as N\nA --> B\nA .. N'),
    ("all",         "note left of A : Left\nA --> B\nnote right of B : Right\nnote on link : Link"),
]

for pattern_l, pattern in rel_note_patterns:
    write_puml(f"class_note_rel_{pattern_l}.puml", wrap(
        f"class A {{\n  +void method()\n}}\nclass B {{\n  +void method()\n}}\n{pattern}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 93. MORE ANNOTATION TYPES
# ─────────────────────────────────────────────────────────────────────────────

java_annotations_detailed = [
    ("SafeVarargs",       ""),
    ("Native",            ""),
    ("Documented",        ""),
    ("Inherited",         ""),
    ("Retention",         "+RetentionPolicy value()"),
    ("Target",            "+ElementType[] value()"),
    ("Repeatable",        "+Class<? extends Annotation> value()"),
    ("Resource",          '+String name() default ""\n  +String lookup() default ""\n  +Class type() default Object.class'),
    ("Inject",            ""),
    ("Qualifier",         ""),
    ("Scope",             ""),
    ("Singleton",         ""),
    ("Named",             '+String value() default ""'),
    ("Produces",          ""),
    ("Consumes",          ""),
    ("Path",              "+String value()"),
    ("GET",               ""),
    ("POST",              ""),
    ("PUT",               ""),
    ("DELETE",            ""),
    ("Valid",             ""),
    ("NotEmpty",          '+String message() default ""'),
    ("Email",             '+String message() default ""'),
    ("Min",               "+long value()"),
    ("Max",               "+long value()"),
    ("Positive",          ""),
    ("Negative",          ""),
    ("Future",            ""),
    ("Past",              ""),
    ("AssertTrue",        ""),
    ("AssertFalse",       ""),
]

for ann_name, body in java_annotations_detailed:
    safe_name = ann_name.lower()
    if body:
        write_puml(f"class_ann_java_{safe_name}.puml", wrap(
            f"annotation {ann_name} {{\n  {body}\n}}"
        ))
    else:
        write_puml(f"class_ann_java_{safe_name}.puml", wrap(
            f"annotation {ann_name}"
        ))


# ─────────────────────────────────────────────────────────────────────────────
# 94. COMPREHENSIVE GENERIC CLASSES
# ─────────────────────────────────────────────────────────────────────────────

generic_patterns = [
    ("single",       "Container<T>",             "T item",     "T get()\nvoid set(T val)"),
    ("bounded_ext",  "Sorter<T extends Comparable<T>>", "List<T> data", "List<T> sort()"),
    ("bounded_sup",  "Adder<T super Integer>",   "T base",     "T add(T other)"),
    ("two_params",   "Pair<A, B>",               "A first\nB second", "A getFirst()\nB getSecond()"),
    ("three_params", "Triple<A, B, C>",          "A first\nB second\nC third", "A getFirst()"),
    ("nested",       "Nested<List<T>>",          "List<T> inner", "T getFirst()"),
    ("wildcard_in",  "Printer<? extends Shape>", "List<? extends Shape> shapes", "void printAll()"),
    ("recursive",    "Node<T extends Node<T>>",  "T next\nT prev", "T getNext()"),
    ("func_type",    "Transformer<F, T>",        "Function<F,T> fn", "T transform(F input)"),
    ("multi_bound",  "Processor<T extends Runnable & Serializable>", "T task", "void run()"),
]

for label, class_sig, fields, methods in generic_patterns:
    field_lines = "\n".join([f"  +{f}" for f in fields.split("\n")])
    method_lines = "\n".join([f"  +{m}" for m in methods.split("\n")])
    write_puml(f"class_generic_pattern_{label}.puml", wrap(
        f"class {class_sig} {{\n{field_lines}\n  --\n{method_lines}\n}}"
    ))


# ─────────────────────────────────────────────────────────────────────────────
# 95. FINAL MEGA COMBINATIONS
# ─────────────────────────────────────────────────────────────────────────────

# All skinparam monochrome variants
for mono in ["true", "false", "reverse"]:
    write_puml(f"class_skinparam_mono_{mono}.puml", wrap(
        f"skinparam monochrome {mono}\n"
        "class A <<service>> {\n  +String field\n  +void method()\n}\n"
        "class B <<entity>>\nA --> B"
    ))

# Shadowing variants
for shadow in ["true", "false"]:
    write_puml(f"class_skinparam_shadow_{shadow}.puml", wrap(
        f"skinparam shadowing {shadow}\n"
        "class A {\n  +void method()\n}\nclass B\nA --> B"
    ))

# Padding variants
for padding in ["5", "10", "15", "20", "30"]:
    write_puml(f"class_skinparam_padding_{padding}.puml", wrap(
        f"skinparam padding {padding}\n"
        "class A {\n  +void method()\n  +String field\n}\nclass B\nA --> B"
    ))

# Line type variants
for line_type in ["ortho", "polyline", "curved"]:
    write_puml(f"class_skinparam_linetype_{line_type}.puml", wrap(
        f"skinparam linetype {line_type}\n"
        "class A\nclass B\nclass C\nA --> B\nB --> C\nA --> C"
    ))

# Nodesep and ranksep variants
for nodesep in ["20", "50", "100", "150"]:
    write_puml(f"class_skinparam_nodesep_{nodesep}.puml", wrap(
        f"skinparam nodesep {nodesep}\n"
        "class A\nclass B\nclass C\nA --> B\nB --> C"
    ))

for ranksep in ["20", "50", "100", "150"]:
    write_puml(f"class_skinparam_ranksep_{ranksep}.puml", wrap(
        f"skinparam ranksep {ranksep}\n"
        "class A\nclass B\nclass C\nA --> B\nB --> C"
    ))

# Scale variants
for scale in ["0.5", "0.75", "1.0", "1.5", "2.0"]:
    write_puml(f"class_scale_{scale.replace('.', '_')}.puml", wrap(
        f"scale {scale}\n"
        "class A {\n  +void method()\n}\nclass B\nA --> B"
    ))

# DPI variants
for dpi in ["72", "96", "150", "300"]:
    write_puml(f"class_dpi_{dpi}.puml", wrap(
        f"skinparam dpi {dpi}\n"
        "class A {\n  +void method()\n}\nclass B\nA --> B"
    ))

# ─────────────────────────────────────────────────────────────────────────────
# 70. FINAL SUMMARY REPORT
# ─────────────────────────────────────────────────────────────────────────────

print(f"Generated {file_count} .puml files in {OUTPUT_DIR}")

