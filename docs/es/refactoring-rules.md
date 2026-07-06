# Reglas de Refactorización

complexipy incluye un sistema de refactorización inspirado en clippy que proporciona sugerencias accionables para reducir la complejidad cognitiva. Cada regla tiene un ID único, categoría y nivel de aplicabilidad.

## Categorías de Reglas

| Categoría          | Icono | Descripción                                              |
| ------------------ | ----- | -------------------------------------------------------- |
| **Complejidad**    | •     | Reglas que reducen directamente la complejidad cognitiva |
| **Legibilidad**    | •     | Reglas que mejoran la legibilidad del código             |
| **Mantenibilidad** | •     | Reglas que mejoran la mantenibilidad a largo plazo       |

## Niveles de Aplicabilidad

| Nivel                 | Icono | Descripción                                                     |
| --------------------- | ----- | --------------------------------------------------------------- |
| **Auto-aplicable**    | \*    | Seguro de aplicar automáticamente sin revisión humana           |
| **Necesita revisión** | !     | Puede ser incorrecto en algunos casos, necesita revisión humana |
| **Informativo**       | i     | Solo orientación, no es directamente accionable                 |

______________________________________________________________________

## Reglas de Complejidad

### C001: Aplanar Condiciones Anidadas

**Categoría:** • Complejidad\
**Aplicabilidad:** ! Necesita revisión\
**Prioridad:** Alta (4/5)

Aplana bloques de condiciones anidadas usando cláusulas de guarda con retornos anticipados.

#### ¿Cuándo se activa?

Esta regla se activa cuando una función tiene sentencias `if` profundamente anidadas (2+ niveles de anidamiento) que añaden complejidad significativa.

#### Ejemplo

**Antes:**

```python
def process_data(data):
    if data:
        if data.is_valid():
            if data.is_ready():
                return process(data)
    return None
```

**Después:**

```python
def process_data(data):
    if not data:
        return None
    if not data.is_valid():
        return None
    if not data.is_ready():
        return None
    return process(data)
```

#### Por qué esto ayuda

Las condiciones profundamente anidadas son difíciles de seguir. Usar cláusulas de guarda con retornos anticipados reduce la carga cognitiva al mantener la ruta principal en un nivel de indentación menor.

______________________________________________________________________

### C002: Guardas de Bucles

**Categoría:** • Complejidad\
**Aplicabilidad:** ! Necesita revisión\
**Prioridad:** Media (3/5)

Usa guardas `continue` al inicio de los bucles para reducir el anidamiento.

#### ¿Cuándo se activa?

Esta regla se activa cuando un bucle contiene sentencias `if` anidadas que podrían convertirse en guardas `continue` anticipadas.

#### Ejemplo

**Antes:**

```python
def process_items(items):
    total = 0
    for item in items:
        if item.active:
            if item.ready:
                total += item.value
    return total
```

**Después:**

```python
def process_items(items):
    total = 0
    for item in items:
        if not item.active:
            continue
        if not item.ready:
            continue
        total += item.value
    return total
```

#### Por qué esto ayuda

Las condiciones anidadas dentro de los bucles añaden indentación innecesaria. Usar guardas `continue` mantiene la lógica principal en un nivel de anidamiento menor y hace que el bucle sea más fácil de seguir.

______________________________________________________________________

### C003: Extraer Función Auxiliar

**Categoría:** • Complejidad\
**Aplicabilidad:** ! Necesita revisión\
**Prioridad:** Media (3/5)

Extrae bloques de código complejos en funciones auxiliares separadas.

#### ¿Cuándo se activa?

Esta regla se activa cuando un bloque de código tiene alta complejidad (6+) y abarca múltiples líneas (5+).

#### Ejemplo

**Antes:**

```python
def process_order(order):
    # Complex validation and processing logic
    if order.items:
        for item in order.items:
            if item.quantity > 0:
                if item.price > 0:
                    total = item.quantity * item.price
                    if total > 100:
                        apply_discount(total)
                    process_item(item)
```

**Después:**

```python
def process_order(order):
    if not order.items:
        return
    for item in order.items:
        process_order_item(item)

def process_order_item(item):
    if item.quantity <= 0 or item.price <= 0:
        return
    total = item.quantity * item.price
    if total > 100:
        apply_discount(total)
    process_item(item)
```

#### Por qué esto ayuda

Los bloques de código complejos deben extraerse en funciones con nombre para mejorar la legibilidad y la capacidad de prueba. La función extraída puede recibir un nombre descriptivo que explique su propósito.

______________________________________________________________________

### C004: Dividir Despachador

**Categoría:** • Complejidad\
**Aplicabilidad:** ! Necesita revisión\
**Prioridad:** Baja (2/5)

Divide cadenas largas de `elif` o sentencias `match` en manejadores separados.

#### ¿Cuándo se activa?

Esta regla se activa cuando:

- Una cadena `if/elif` tiene 3+ ramas
- Una sentencia `match` tiene 4+ casos

#### Ejemplo

**Antes:**

```python
def handle_action(action):
    if action == "create":
        return create_resource()
    elif action == "read":
        return read_resource()
    elif action == "update":
        return update_resource()
    elif action == "delete":
        return delete_resource()
    return None
```

**Después:**

```python
def handle_action(action):
    handlers = {
        "create": create_resource,
        "read": read_resource,
        "update": update_resource,
        "delete": delete_resource,
    }
    handler = handlers.get(action)
    return handler() if handler else None
```

#### Por qué esto ayuda

Las cadenas condicionales largas son difíciles de mantener y extender. Dividirlas en manejadores separados hace que cada caso sea independientemente testeable y la lógica de despacho más clara.

______________________________________________________________________

### C006: Reducir Profundidad de Anidamiento

**Categoría:** • Complejidad\
**Aplicabilidad:** * Auto-aplicable\
**Prioridad:** Alta (4/5)

Reduce la profundidad de anidamiento usando retornos anticipados y cláusulas de guarda.

#### ¿Cuándo se activa?

Esta regla se activa cuando el código tiene anidamiento profundo (3+ niveles) que dificulta su seguimiento.

#### Ejemplo

**Antes:**

```python
def validate(user):
    if user.is_active:
        if user.has_permission:
            if user.is_verified:
                return check_access(user)
    return False
```

**Después:**

```python
def validate(user):
    if not user.is_active:
        return False
    if not user.has_permission:
        return False
    if not user.is_verified:
        return False
    return check_access(user)
```

#### Por qué esto ayuda

El anidamiento profundo (3+ niveles) hace que el código sea difícil de seguir. Considera extraer los bloques internos o usar cláusulas de guarda para mantener la indentación superficial.

______________________________________________________________________

### C011: Aplanar Try/Except

**Categoría:** • Complejidad\
**Aplicabilidad:** ! Necesita revisión\
**Prioridad:** Baja (2/5)

Aplana bloques try/except anidados combinándolos o reestructurándolos.

#### ¿Cuándo se activa?

Esta regla se activa cuando los bloques try/except están anidados unos dentro de otros.

#### Ejemplo

**Antes:**

```python
def read_config(path):
    try:
        with open(path) as f:
            try:
                return json.load(f)
            except json.JSONDecodeError:
                return default_config()
    except FileNotFoundError:
        return default_config()
```

**Después:**

```python
def read_config(path):
    try:
        with open(path) as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return default_config()
```

#### Por qué esto ayuda

Los bloques try/except anidados son confusos y difíciles de mantener. Considera fusionarlos o extraer el bloque interno en una función separada con su propio manejo de errores.

______________________________________________________________________

## Reglas de Legibilidad

### C005: Extraer Predicado

**Categoría:** • Legibilidad\
**Aplicabilidad:** ! Necesita revisión\
**Prioridad:** Media (3/5)

Extrae condiciones booleanas complejas en funciones predicado con nombre.

#### ¿Cuándo se activa?

Esta regla se activa cuando una condición booleana contiene 2+ operadores lógicos (and, or, not).

#### Ejemplo

**Antes:**

```python
def is_eligible(user, order):
    if (user.is_active and user.has_subscription) or (order.total > 100 and not order.is_gift):
        return True
    return False
```

**Después:**

```python
def is_eligible(user, order):
    return has_active_subscription(user) or is_qualifying_order(order)

def has_active_subscription(user):
    return user.is_active and user.has_subscription

def is_qualifying_order(order):
    return order.total > 100 and not order.is_gift
```

#### Por qué esto ayuda

Las expresiones booleanas complejas son difíciles de entender a simple vista. Extraerlas en predicados con nombre hace que el código sea autoexplicativo y más fácil de probar.

______________________________________________________________________

## Uso de las Reglas de Refactorización

### Línea de Comandos

```bash
# Muestra sugerencias de refactorización
complexipy . --suggest-refactors

# Muestra sugerencias solo para funciones fallidas
complexipy . --failed --suggest-refactors

# Exporta sugerencias a JSON
complexipy . --output-format json --suggest-refactors
```

### API de Python

```python
from complexipy import code_complexity

code = """
def process(data):
    if data:
        if data.is_valid():
            return process(data)
    return None
"""

result = code_complexity(code)
for func in result.functions:
    for plan in func.refactor_plans:
        print(f"[{plan.rule_id}] {plan.title}")
        print(f"  Category: {plan.category}")
        print(f"  Applicability: {plan.applicability}")
        print(f"  Reduction: -{plan.estimated_reduction} complexity")
        if plan.before_code:
            print(f"  Before:\n{plan.before_code.text}")
        if plan.after_code:
            print(f"  After:\n{plan.after_code.text}")
```

### Salida JSON

La salida JSON incluye todos los metadatos de las reglas para consumo programático:

```json
{
  "rule_id": "C001",
  "kind": "flatten_condition",
  "title": "Flatten nested condition block with guard clauses",
  "category": "Complexity",
  "applicability": "MachineApplicable",
  "description": "Flatten nested condition blocks by using guard clauses with early returns",
  "line_start": 10,
  "line_end": 15,
  "current_complexity": 12,
  "estimated_reduction": 4,
  "estimated_complexity_after": 8,
  "before_code": {
    "text": "if data:\n    if data.is_valid():\n        return process(data)",
    "line_start": 10,
    "line_end": 12
  },
  "after_code": {
    "text": "if not data:\n    return None\nif not data.is_valid():\n    return None\nreturn process(data)",
    "line_start": 10,
    "line_end": 14
  },
  "explanation": "Deeply nested conditions are hard to follow...",
  "references": [
    "https://rohaquinlop.github.io/complexipy/refactoring-rules/#c001-flatten-nested-conditions"
  ]
}
```

______________________________________________________________________

## Referencia de IDs de Reglas

| ID                                               | Nombre                             | Categoría     | Aplicabilidad       | Prioridad |
| ------------------------------------------------ | ---------------------------------- | ------------- | ------------------- | --------- |
| [C001](#c001-aplanar-condiciones-anidadas)       | Aplanar Condiciones Anidadas       | • Complejidad | ! Necesita revisión | Alta      |
| [C002](#c002-guardas-de-bucles)                  | Guardas de Bucles                  | • Complejidad | ! Necesita revisión | Media     |
| [C003](#c003-extraer-funcion-auxiliar)           | Extraer Función Auxiliar           | • Complejidad | ! Necesita revisión | Media     |
| [C004](#c004-dividir-despachador)                | Dividir Despachador                | • Complejidad | ! Necesita revisión | Baja      |
| [C005](#c005-extraer-predicado)                  | Extraer Predicado                  | • Legibilidad | ! Necesita revisión | Media     |
| [C006](#c006-reducir-profundidad-de-anidamiento) | Reducir Profundidad de Anidamiento | • Complejidad | \* Auto-aplicable   | Alta      |
| [C011](#c011-aplanar-tryexcept)                  | Aplanar Try/Except                 | • Complejidad | ! Necesita revisión | Baja      |
