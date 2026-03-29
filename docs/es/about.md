# Acerca de complexipy

## Orígenes e Inspiración

complexipy está inspirado en el artículo de investigación [Cognitive Complexity](https://www.sonarsource.com/resources/cognitive-complexity/) de **G. Ann Campbell** de SonarSource. Este trabajo pionero introdujo un nuevo enfoque para medir la complejidad del código que se alinea mejor con la comprensión humana.

!!! info "Proyecto Independiente"
    Aunque complexipy implementa la metodología de complejidad cognitiva descrita en la investigación de Campbell, **no está afiliado ni respaldado por SonarSource ni por los productos Sonar**. complexipy es una implementación independiente de código abierto, escrita en Rust para el análisis de código Python.

## ¿Por Qué la Complejidad Cognitiva?

La complejidad ciclomática mide la densidad estructural, de pruebas y de ramas del código (número de caminos) contando los puntos de decisión. Sin embargo, esto no refleja cómo los seres humanos realmente entienden el código. La complejidad cognitiva mide qué tan difícil es para los seres humanos entender y mantener el código, penalizando el anidamiento, las interrupciones de flujo y la lógica difícil de comprender.

### Diferentes Enfoques para Medir la Complejidad

La complejidad ciclomática (McCabe, 1976) cuenta cada punto de decisión:

```python
# Complejidad ciclomática: 4
def example1(a, b, c):
    if a:
        return 1
    if b:
        return 2
    if c:
        return 3
    return 4
```

```python
# Complejidad ciclomática: 4 (¡igual que arriba!)
def example2(a, b, c):
    if a:
        if b:
            if c:
                return 1
    return 4
```

Ambas funciones tienen la misma complejidad ciclomática, pero cualquier desarrollador te dirá que `example2` es mucho más difícil de entender.

### La Solución de Complejidad Cognitiva

La complejidad cognitiva tiene en cuenta el anidamiento:

```python
# Complejidad cognitiva: 3
def example1(a, b, c):
    if a:        # +1
        return 1
    if b:        # +1
        return 2
    if c:        # +1
        return 3
    return 4
```

```python
# Complejidad cognitiva: 6
def example2(a, b, c):
    if a:        # +1
        if b:    # +2 (1 + nesting_level)
            if c:  # +3 (1 + nesting_level)
                return 1
    return 4
```

Esto se alinea con la intuición humana: el código anidado requiere mantener más contexto en la mente.

## Objetivos del Proyecto

1. **Rendimiento** - Análisis ultrarrápido usando Rust
2. **Precisión** - Implementación fiel de los principios de complejidad cognitiva
3. **Accesibilidad** - Integración sencilla con los flujos de trabajo de desarrollo en Python
4. **Accionabilidad** - Perspectivas claras y accionables para mejorar la calidad del código

## El Equipo

complexipy está desarrollado con ❤️ por [@rohaquinlop](https://github.com/rohaquinlop) y [colaboradores](https://github.com/rohaquinlop/complexipy/graphs/contributors).

## Licencia

complexipy se publica bajo la [Licencia MIT](https://github.com/rohaquinlop/complexipy/blob/main/LICENSE).
